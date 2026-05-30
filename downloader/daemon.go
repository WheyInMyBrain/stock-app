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

// 🎯 ANSI COLOR CODES FOR PERFECT TERMINAL SCANNABILITY
const (
	ColorBlue  = "\033[34m"
	ColorCyan  = "\033[36m"
	ColorReset = "\033[0m"
)

func RunPersistentDaemonMode(workerCount int, globalDataDir string) {
	// Force standard output to be completely unbuffered to prevent block delays inside Tauri
	os.Stdout = os.NewFile(uintptr(1), "/dev/stdout") 

	// 🎯 CYAN LOGS FOR INITIALIZATION & ENGINE STATE BOOTS
	fmt.Printf("%s🔥 [GO ENGINE]: Launching persistent background network layer...%s\n", ColorCyan, ColorReset)
	fmt.Printf("%s[engine] 🕵️‍♂️ Initializing organic master session handshakes...%s\n", ColorCyan, ColorReset)

	nseClient, nseErr := scrape_nse.NewNSEClient()
	if nseErr != nil {
		fmt.Fprintf(os.Stderr, "❌ Critical: Master NSE Handshake failed on app load: %v\n", nseErr)
	}

	bseClient, bseErr := scrape_bse.NewBSEClient()
	if bseErr != nil {
		fmt.Fprintf(os.Stderr, "❌ Critical: Master BSE Handshake failed on app load: %v\n", bseErr)
	}

	fmt.Printf("%s🏁 [GO ENGINE SUCCESS]: Network state is fully hot. Awaiting incoming ticker signals...%s\n", ColorCyan, ColorReset)
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
				fmt.Println("📟 [GO DAEMON ERROR]: Malformed tracking command payload.")
				os.Stdout.Sync()
				continue
			}

			ticker := parts[1]
			mode := strings.ToLower(parts[2])
			targetApi := parts[3]

			// Capture precise start metric boundary
			startTime := time.Now() 

			// 🎯 DEEP BLUE COLS FOR REPEATED TICKER DATA ENGINE PIPELINES
			fmt.Printf("\n%s=== 🚀 Starting Multi-Exchange Extraction Engine for: %s ===%s\n", ColorBlue, ticker, ColorReset)
			fmt.Printf("%s⚡ [HOT REFRESH TRIGGERED]: Executing pipeline instantly for %s (%s -> %s)%s\n", ColorBlue, ticker, mode, targetApi, ColorReset)
			os.Stdout.Sync()

			if (mode == "nse" || mode == "both") && nseClient != nil {
				_ = scrape_nse.ExecuteWithWarmClient(nseClient, ticker, workerCount, targetApi, globalDataDir)
			}
			if (mode == "bse" || mode == "both") && bseClient != nil {
				_ = scrape_bse.ExecuteWithWarmClient(bseClient, ticker, workerCount, targetApi, globalDataDir)
			}

			// 🎯 DEEP BLUE LOGS FOR COMPLETED WORKSPACE TIMERS
			fmt.Printf("%s=== 🎉 [%s] Pipelines Completed in %v ===%s\n\n", ColorBlue, ticker, time.Since(startTime), ColorReset)
			os.Stdout.Sync()

			// Emit completion token wrapper back to Rust bridge receiver
			fmt.Printf("SIGNAL_COMPLETED:%s:%s\n", ticker, targetApi)
			os.Stdout.Sync() 
		}
	}
}