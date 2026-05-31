package main

import (
    "bufio"
    "fmt"
    "net"
    "os"
    "strings"
    "time"
    "downloader/scrape_bse"
    "downloader/scrape_nse"
)

const (
    ColorBlue  = "\033[34m"
    ColorCyan  = "\033[36m"
    ColorReset = "\033[0m"
)

func RunPersistentDaemonMode(workerCount int, globalDataDir string) {
    os.Stdout = os.NewFile(uintptr(1), "/dev/stdout") 

    fmt.Printf("%s🔥 [GO ENGINE]: Launching persistent background network layer...%s\n", ColorCyan, ColorReset)
    
    nseClient, nseErr := scrape_nse.NewNSEClient()
    if nseErr != nil {
        fmt.Fprintf(os.Stderr, "❌ Critical: Master NSE Handshake failed on app load: %v\n", nseErr)
    }

    bseClient, bseErr := scrape_bse.NewBSEClient()
    if bseErr != nil {
        fmt.Fprintf(os.Stderr, "❌ Critical: Master BSE Handshake failed on app load: %v\n", bseErr)
    }

    ipcServer, err := NewIPCServer(globalDataDir)
    if err != nil {
        fmt.Fprintf(os.Stderr, "❌ CRITICAL IPC SEED FAULT: %v\n", err)
        os.Stdout.Sync()
        os.Exit(1)
    }
    defer ipcServer.Close()

    fmt.Printf("%s🏁 [GO ENGINE SUCCESS]: IPC Socket Server listening at: %s%s\n", ColorCyan, ipcServer.socketPath, ColorReset)
    os.Stdout.Sync()

    for {
        conn, err := ipcServer.listener.Accept()
        if err != nil {
            fmt.Fprintf(os.Stderr, "🚨 [IPC CONNECTION ERROR]: Failed accepting socket handshake: %v\n", err)
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
                if len(parts) < 4 {
                    return
                }

                mode := strings.ToLower(parts[1])
                targetApi := parts[2]
                
                var fromTimeStr string
                var ticker string
                isStreamMode := false

                for _, part := range parts[3:] {
                    if strings.HasPrefix(part, "-from=") || strings.HasPrefix(part, "--from=") {
                        fromTimeStr = strings.Split(part, "=")[1]
                    } else if part == "--stream" {
                        isStreamMode = true
                    } else if !strings.HasPrefix(part, "--metadata_module=") {
                        ticker = part
                    }
                }

                if fromTimeStr == "" {
                    fromTimeStr = fmt.Sprintf("%d", time.Now().Unix())
                }

                if ticker == "" {
                    return
                }

                startTime := time.Now() 
                fmt.Printf("\n%s=== 🚀 IPC Connection Accepted: Fetching metrics for: %s ===%s\n", ColorBlue, ticker, ColorReset)
                os.Stdout.Sync()

                var rawJSONPayload string
                var pipelineErr error

                if (mode == "nse" || mode == "both") && nseClient != nil {
                    rawJSONPayload, pipelineErr = scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir, fromTimeStr)
                }
                
                if pipelineErr == nil && (mode == "bse" || mode == "both") && bseClient != nil {
                    bsePayload, bseErr := scrape_bse.ExecuteWithWarmClient(bseClient, ticker, workerCount, targetApi, globalDataDir)
                    if bseErr != nil {
                        pipelineErr = bseErr
                    } else if bsePayload != "" {
                        rawJSONPayload = bsePayload
                    }
                }

                if pipelineErr != nil {
                    fmt.Fprintf(os.Stderr, "🚨 [IPC FETCH FAULT]: Pipeline error during lookup: %v\n", pipelineErr)
                    return
                }

                if isStreamMode && rawJSONPayload != "" {
                    fmt.Printf("%s⚡ [IPC BINARY PIPE ACTIVE]: Transmitting framed byte matrix stream directly over socket...%s\n", ColorCyan, ColorReset)
                    os.Stdout.Sync()

                    if writeErr := WriteFramedPayload(c, rawJSONPayload); writeErr != nil {
                        fmt.Fprintf(os.Stderr, "🚨 [IPC WRITE CRITICAL FAULT]: Failed writing bytes down frame channel: %v\n", writeErr)
                    }
                }

                fmt.Printf("%s=== 🎉 [%s] Pipelines Completed in %v over IPC Socket ===%s\n\n", ColorBlue, ticker, time.Since(startTime), ColorReset)
                fmt.Printf("SIGNAL_COMPLETED:%s:%s\n", ticker, targetApi)
                os.Stdout.Sync() 
            }
        }(conn)
    }
}