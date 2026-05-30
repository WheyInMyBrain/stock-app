// stock-app/downloader/daemon.go

package main

import (
	"bufio"
	"fmt"
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
	// Force standard output to be completely unbuffered to prevent pipe stalls inside Tauri
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

	fmt.Printf("%s🏁 [GO ENGINE SUCCESS]: Network state is fully hot. Awaiting signals...%s\n", ColorCyan, ColorReset)
	os.Stdout.Sync()

	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" {
			continue
		}

		if strings.HasPrefix(line, "RUN ") {
			parts := strings.Fields(line)
			if len(parts) < 4 {
				continue
			}

			ticker := parts[1]
			mode := strings.ToLower(parts[2])
			targetApi := parts[3]

			// 🎯 CHECK FOR THE OPTIONAL STREAM ARGUMENT
			isStreamMode := false
			if len(parts) >= 5 && parts[4] == "--stream" {
				isStreamMode = true
			}

			startTime := time.Now() 
			fmt.Printf("\n%s=== 🚀 Starting Multi-Exchange Extraction Engine for: %s ===%s\n", ColorBlue, ticker, ColorReset)
			os.Stdout.Sync()

			var rawJSONPayload string
			var err error

			// 🎯 EXECUTE THE REAL NETWORK PIPELINES CAPTURING RAW STRINGS
			if (mode == "nse" || mode == "both") && nseClient != nil {
				rawJSONPayload, err = scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir)
			}
			
			// If running mode is BSE or both, and the NSE pass didn't hit a fatal error, pool the BSE pipeline
			if err == nil && (mode == "bse" || mode == "both") && bseClient != nil {
				bsePayload, bseErr := scrape_bse.ExecuteWithWarmClient(bseClient, ticker, workerCount, targetApi, globalDataDir)
				if bseErr != nil {
					err = bseErr
				} else if bsePayload != "" {
					rawJSONPayload = bsePayload
				}
			}

			if err != nil {
				fmt.Fprintf(os.Stderr, "🚨 [FETCH FAULT]: Pipeline error during lookup: %v\n", err)
				os.Stdout.Sync()
				continue
			}

			// 🎯 IF STREAM MODE IS ACTIVE, FLASH THE TRUE INTERCEPTED JSON DATA THROUGH STDOUT
			if isStreamMode && rawJSONPayload != "" {
				fmt.Printf("%s⚡ [RAM PASS THROUGH ACTIVE]: Routing payload via memory stream for fast UI render...%s\n", ColorCyan, ColorReset)
				os.Stdout.Sync()

				// Stream raw server data down the warm stdout pipe into Rust's memory accumulator
				fmt.Printf("PAYLOAD_START:%s:%s\n%s\nPAYLOAD_END\n", ticker, targetApi, rawJSONPayload)
				os.Stdout.Sync()
			}

			fmt.Printf("%s=== 🎉 [%s] Pipelines Completed in %v ===%s\n\n", ColorBlue, ticker, time.Since(startTime), ColorReset)
			fmt.Printf("SIGNAL_COMPLETED:%s:%s\n", ticker, targetApi)
			os.Stdout.Sync() 
		}
	}
}