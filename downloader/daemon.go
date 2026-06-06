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
    ColorRed   = "\033[31m"
    ColorReset = "\033[0m"
)

func RunPersistentDaemonMode(workerCount int, globalDataDir string) {
    // Platform-neutral standard descriptor initialization override
    os.Stdout = os.NewFile(uintptr(1), "stdout") 

    fmt.Printf("%s[GO-downloader] 🔥 [GO ENGINE]: Launching persistent background network layer...%s\n", ColorBlue, ColorReset)
    
    nseClient, nseErr := scrape_nse.NewNSEClient()
    if nseErr != nil {
        fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Critical: Master NSE Handshake failed on app load: %v%s\n", ColorRed, nseErr, ColorReset)
    }

    bseClient, bseErr := scrape_bse.NewBSEClient()
    if bseErr != nil {
        fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Critical: Master BSE Handshake failed on app load: %v%s\n", ColorRed, bseErr, ColorReset)
    }

    ipcServer, err := NewIPCServer(globalDataDir)
    if err != nil {
        fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ CRITICAL IPC SEED FAULT: %v%s\n", ColorRed, err, ColorReset)
        os.Stdout.Sync()
        os.Exit(1)
    }
    defer ipcServer.Close()

    fmt.Printf("%s[GO-downloader] 🏁 [GO ENGINE SUCCESS]: IPC Socket Server listening at: %s%s\n", ColorBlue, ipcServer.socketPath, ColorReset)
    os.Stdout.Sync()

    for {
        conn, err := ipcServer.listener.Accept()
        if err != nil {
            fmt.Fprintf(os.Stderr, "%s[GO-downloader] 🚨 [IPC CONNECTION ERROR]: Failed accepting socket handshake: %v%s\n", ColorRed, err, ColorReset)
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
                onlyJsonMode := false

                // Scan all incoming parameters dynamically over the active Unix Domain Connection
                for _, part := range parts[3:] {
                    if strings.HasPrefix(part, "-from=") || strings.HasPrefix(part, "--from=") {
                        fromTimeStr = strings.Split(part, "=")[1]
                    } else if part == "--stream" {
                        isStreamMode = true
                    } else if part == "--only-json" || part == "-only-json" {
                        onlyJsonMode = true
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
                fmt.Printf("\n%s[GO-downloader] === 🚀 IPC Connection Accepted: Fetching metrics for: %s ===%s\n", ColorBlue, ticker, ColorReset)
                os.Stdout.Sync()

                var rawJSONPayload string
                var pipelineErr error

                // Forward onlyJsonMode parameter directly down into your active execution strategies
                if (mode == "nse" || mode == "both") && nseClient != nil {
                    rawJSONPayload, pipelineErr = scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir, fromTimeStr, onlyJsonMode)
                }
                
                if pipelineErr == nil && (mode == "bse" || mode == "both") && bseClient != nil {
                    bsePayload, bseErr := scrape_bse.ExecuteWithWarmClient(bseClient, ticker, workerCount, targetApi, globalDataDir, onlyJsonMode)
                    if bseErr != nil {
                        pipelineErr = bseErr
                    } else if bsePayload != "" {
                        rawJSONPayload = bsePayload
                    }
                }

                if pipelineErr != nil {
                    fmt.Fprintf(os.Stderr, "%s[GO-downloader] 🚨 [IPC FETCH FAULT]: Pipeline error during lookup: %v%s\n", ColorRed, pipelineErr, ColorReset)
                    return
                }

                if isStreamMode && rawJSONPayload != "" {
                    fmt.Printf("%s[GO-downloader] ⚡ [IPC BINARY PIPE ACTIVE]: Transmitting framed byte matrix stream directly over socket...%s\n", ColorBlue, ColorReset)
                    os.Stdout.Sync()

                    if writeErr := WriteFramedPayload(c, rawJSONPayload); writeErr != nil {
                        fmt.Fprintf(os.Stderr, "%s[GO-downloader] 🚨 [IPC WRITE CRITICAL FAULT]: Failed writing bytes down frame channel: %v%s\n", ColorRed, writeErr, ColorReset)
                    }
                }

                fmt.Printf("%s[GO-downloader] === 🎉 [%s] Pipelines Completed in %v over IPC Socket ===%s\n\n", ColorBlue, ticker, time.Since(startTime), ColorReset)
                
                // ⚠️ CRITICAL HANDSHAKE CORE: Kept exactly intact without color text wrappers 
                // to protect Rust's structural stream tracking filters!
                fmt.Printf("SIGNAL_COMPLETED:%s:%s\n", ticker, targetApi)
                os.Stdout.Sync() 
            }
        }(conn)
    }
}