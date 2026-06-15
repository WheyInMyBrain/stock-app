package main

import (
    "bufio"
    "fmt"
    "net"
    "os"
    "strings"
    "sync"
    "time"
    "downloader/scrape_bse"
    "downloader/scrape_nse"
)

const (
    ColorBlue  = "\033[96m"
    ColorRed   = "\033[31m"
    ColorReset = "\033[0m"
)

// SocketSafeWriter wraps a network connection with a mutex. This ensures that concurrent 
// worker threads can report progress down the active IPC link simultaneously without 
// interleaving or corrupting the text transmission lines.
type SocketSafeWriter struct {
    Mu   sync.Mutex
    Conn net.Conn
}

// WriteLine safely serializes strings down the open socket channel followed by a newline separator
func (sw *SocketSafeWriter) WriteLine(line string) {
    sw.Mu.Lock()
    defer sw.Mu.Unlock()
    if sw.Conn != nil {
        fmt.Fprint(sw.Conn, line+"\n")
    }
}

func RunPersistentDaemonMode(workerCount int, globalDataDir string) {
    // Platform-neutral standard descriptor initialization override
    os.Stdout = os.NewFile(uintptr(1), "stdout") 

    fmt.Printf("%s 🔥 [GO ENGINE]: Launching persistent background network layer...%s\n", ColorBlue, ColorReset)
    
    nseClient, nseErr := scrape_nse.NewNSEClient()
    if nseErr != nil {
        fmt.Fprintf(os.Stderr, "%s ❌ Critical: Master NSE Handshake failed on app load: %v%s\n", ColorRed, nseErr, ColorReset)
    }

    bseClient, bseErr := scrape_bse.NewBSEClient()
    if bseErr != nil {
        fmt.Fprintf(os.Stderr, "%s ❌ Critical: Master BSE Handshake failed on app load: %v%s\n", ColorRed, bseErr, ColorReset)
    }

    ipcServer, err := NewIPCServer(globalDataDir)
    if err != nil {
        fmt.Fprintf(os.Stderr, "%s ❌ CRITICAL IPC SEED FAULT: %v%s\n", ColorRed, err, ColorReset)
        os.Stdout.Sync()
        os.Exit(1)
    }
    defer ipcServer.Close()

    fmt.Printf("%s 🏁 [GO ENGINE SUCCESS]: IPC Socket Server listening at: %s%s\n", ColorBlue, ipcServer.socketPath, ColorReset)
    os.Stdout.Sync()

    for {
        conn, err := ipcServer.listener.Accept()
        if err != nil {
            fmt.Fprintf(os.Stderr, "%s 🚨 [IPC CONNECTION ERROR]: Failed accepting socket handshake: %v%s\n", ColorRed, err, ColorReset)
            continue
        }

        go func(c net.Conn) {
            defer c.Close()

            scanner := bufio.NewScanner(c)
            if scanner.Scan() {
                line := scanner.Text()
                if !strings.HasPrefix(line, "RUN ") {
                    return
                }

                parts := strings.Fields(line)
                if len(parts) < 2 { 
                    return
                }

                mode := "both"
                targetApi := "" 
                var fromTimeStr string
                var ticker string
                isStreamMode := false
                onlyJsonMode := false

                // DYNAMIC SCANNER LOOP: Gathers arguments independent of order/position
                for _, part := range parts[1:] {
                    if strings.HasPrefix(part, "--mode=") || strings.HasPrefix(part, "-mode=") {
                        mode = strings.ToLower(strings.Split(part, "=")[1])
                    } else if strings.HasPrefix(part, "--api=") || strings.HasPrefix(part, "-api=") {
                        targetApi = strings.Split(part, "=")[1]
                    } else if strings.HasPrefix(part, "-from=") || strings.HasPrefix(part, "--from=") {
                        fromTimeStr = strings.Split(part, "=")[1]
                    } else if part == "--stream" || part == "-stream" {
                        isStreamMode = true
                    } else if part == "--only-json" || part == "-only-json" || part == "only-json" {
                        onlyJsonMode = true
                    } else if !strings.HasPrefix(part, "-") && !strings.HasPrefix(part, "--metadata_module=") {
                        ticker = part
                    }
                }

                if fromTimeStr == "" {
                    fromTimeStr = fmt.Sprintf("%d", time.Now().Unix())
                }

                if ticker == "" {
                    fmt.Fprintf(os.Stderr, "%s 🚨 [IPC PARSE FAULT]: Wire payload ignored. No target stock ticker detected.%s\n", ColorRed, ColorReset)
                    return
                }

                startTime := time.Now() 
                fmt.Printf("\n%s === 🚀 IPC Connection Accepted: Fetching metrics for: %s ===%s\n", ColorBlue, ticker, ColorReset)
                os.Stdout.Sync()

                var wg sync.WaitGroup
                var mu sync.Mutex // 🎯 Protects shared variables from parallel write data races
                var rawJSONPayload string
                var pipelineErr error

                // 🎯 INSTANTIATE TELEMETRY LINK: 
                telemetryWriter := &SocketSafeWriter{Conn: c}

                // 🚀 PARALLEL INGESTION: Fire both exchange pipelines concurrently since they target completely independent networks
                if (mode == "nse" || mode == "both") && nseClient != nil {
                    wg.Add(1)
                    go func() {
                        defer wg.Done()
                        nsePayload, nseErr := scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir, fromTimeStr, onlyJsonMode, telemetryWriter)
                        
                        mu.Lock()
                        defer mu.Unlock()
                        if nseErr != nil {
                            fmt.Fprintf(os.Stderr, "%s 🚨 [NSE CRITICAL BATCH FAULT]: %v%s\n", ColorRed, nseErr, ColorReset)
                            pipelineErr = nseErr
                        } else if nsePayload != "" {
                            rawJSONPayload = nsePayload
                        }
                    }()
                }

                if (mode == "bse" || mode == "both") && bseClient != nil {
                    wg.Add(1)
                    go func() {
                        defer wg.Done()
                        bsePayload, bseErr := scrape_bse.ExecuteWithWarmClient(bseClient, ticker, workerCount, targetApi, globalDataDir, onlyJsonMode, telemetryWriter)
                        
                        mu.Lock()
                        defer mu.Unlock()
                        if bseErr != nil {
                            fmt.Fprintf(os.Stderr, "%s 🚨 [BSE CRITICAL BATCH FAULT]: %v%s\n", ColorRed, bseErr, ColorReset)
                            if pipelineErr == nil {
                                pipelineErr = bseErr
                            }
                        } else if bsePayload != "" {
                            rawJSONPayload = bsePayload
                        }
                    }()
                }

                // Block and wait for both parallel background network branches to complete operations
                wg.Wait()

                if pipelineErr != nil {
                    fmt.Fprintf(os.Stderr, "%s 🚨 [IPC FETCH FAULT]: Pipeline error during lookup: %v%s\n", ColorRed, pipelineErr, ColorReset)
                    return
                }

                if isStreamMode && rawJSONPayload != "" {
                    fmt.Printf("%s ⚡ [IPC BINARY PIPE ACTIVE]: Transmitting framed byte matrix stream directly over socket...%s\n", ColorBlue, ColorReset)
                    os.Stdout.Sync()

                    telemetryWriter.WriteLine("PAYLOAD_START")

                    if writeErr := WriteFramedPayload(c, rawJSONPayload); writeErr != nil {
                        fmt.Fprintf(os.Stderr, "%s 🚨 [IPC WRITE CRITICAL FAULT]: Failed writing bytes down frame channel: %v%s\n", ColorRed, writeErr, ColorReset)
                    }
                }

                fmt.Printf("%s === 🎉 [%s] Pipelines Completed in %v over IPC Socket ===%s\n\n", ColorBlue, ticker, time.Since(startTime), ColorReset)
                
                fmt.Printf("SIGNAL_COMPLETED:%s:%s\n", ticker, targetApi)
                os.Stdout.Sync() 
            }
        }(conn)
    }
}