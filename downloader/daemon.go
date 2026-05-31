// stock-app/downloader/daemon.go

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
    // Force standard output to be completely unbuffered to prevent pipe stalls inside Tauri logs
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

    // 🎯 STEP 1: INITIALIZE THE LOCAL IPC SOCKET SERVER
    ipcServer, err := NewIPCServer(globalDataDir)
    if err != nil {
        fmt.Fprintf(os.Stderr, "❌ CRITICAL IPC SEED FAULT: %v\n", err)
        os.Stdout.Sync()
        os.Exit(1)
    }
    defer ipcServer.Close()

    fmt.Printf("%s🏁 [GO ENGINE SUCCESS]: IPC Socket Server listening at: %s%s\n", ColorCyan, ipcServer.socketPath, ColorReset)
    os.Stdout.Sync()

    // 🎯 STEP 2: MULTI-CONNECTION SOCKET ACCEPT LOOP
    for {
        conn, err := ipcServer.listener.Accept()
        if err != nil {
            fmt.Fprintf(os.Stderr, "🚨 [IPC CONNECTION ERROR]: Failed accepting socket handshake: %v\n", err)
            continue
        }

        // Handle each incoming connection context concurrently so your data remains non-blocking
        go func(c net.Conn) {
            defer c.Close()

            // Read the single-line execution instruction string sent by Rust over the connection
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

                ticker := parts[1]
                mode := strings.ToLower(parts[2])
                targetApi := parts[3]

                isStreamMode := false
                if len(parts) >= 5 && parts[4] == "--stream" {
                    isStreamMode = true
                }

                startTime := time.Now() 
                fmt.Printf("\n%s=== 🚀 IPC Connection Accepted: Fetching metrics for: %s ===%s\n", ColorBlue, ticker, ColorReset)
                os.Stdout.Sync()

                var rawJSONPayload string
                var pipelineErr error

                // 🎯 CALCULATE THE CURRENT TIMESTAMP RIGHT HERE FOR THE PIPELINE
                nowTimeStr := fmt.Sprintf("%d", time.Now().Unix())

                // Fetch true live string data arrays from your scraping engine sub-modules
                if (mode == "nse" || mode == "both") && nseClient != nil {
                    rawJSONPayload, pipelineErr = scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir, nowTimeStr)
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

                // 🎯 STEP 3: BLAST TRUE RAW DATA OVER SECURE LOCAL IPC BACKEND LINK
                if isStreamMode && rawJSONPayload != "" {
                    fmt.Printf("%s⚡ [IPC BINARY PIPE ACTIVE]: Transmitting framed byte matrix stream directly over socket...%s\n", ColorCyan, ColorReset)
                    os.Stdout.Sync()

                    // Ship framed binary data packages out of stdout entirely and safely over our local socket link
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