package main

import (
	"flag"
	"fmt"
	"os"
	"stock-app/downloader/scrape_bse"
	"stock-app/downloader/scrape_nse"
	"strings"
	"time"
)

func main() {
	// 1. Define command-line flags
	workerCountPtr := flag.Int("workers", 5, "Number of concurrent background download workers")
	modePtr := flag.String("mode", "both", "Target exchange execution strategy: 'nse', 'bse', or 'both'")

	// 2. Custom usage text block if arguments are missing or broken
	flag.Usage = func() {
		fmt.Printf("Usage: go run ./downloader/main.go [options] SYMBOL\n\n")
		fmt.Printf("Options:\n")
		flag.PrintDefaults()
		fmt.Printf("\nExamples:\n")
		fmt.Printf("  go run ./downloader/main.go IMFA               (Scrapes both exchanges via 5 default workers)\n")
		fmt.Printf("  go run ./downloader/main.go -mode=nse IMFA     (Extracts data from NSE only)\n")
		fmt.Printf("  go run ./downloader/main.go -mode=bse -workers=8 IMFA (Extracts data from BSE only with 8 workers)\n")
	}

	// 3. Parse options from the raw command execution line
	flag.Parse()

	// 4. Extract target positional symbol
	args := flag.Args()
	if len(args) < 1 {
		fmt.Println("❌ Error: Missing stock symbol argument.")
		flag.Usage()
		os.Exit(1)
	}

	ticker := args[0]
	workerCount := *workerCountPtr
	mode := strings.ToLower(strings.TrimSpace(*modePtr))
	startTime := time.Now()

	// Validation guard for the execution mode flag
	if mode != "nse" && mode != "bse" && mode != "both" {
		fmt.Printf("❌ Error: Invalid execution mode '%s'. Choose between 'nse', 'bse', or 'both'.\n", mode)
		os.Exit(1)
	}

	fmt.Printf("=== 🚀 Starting Multi-Exchange Extraction Engine for: %s (Workers: %d, Mode: %s) ===\n", ticker, workerCount, mode)

	// ============================================================================
	// PHASE 1: EXECUTE NATIONAL STOCK EXCHANGE (NSE) ENGINE DATA STREAM
	// ============================================================================
	if mode == "nse" || mode == "both" {
		fmt.Printf("\n[engine] 🟢 Triggering National Stock Exchange (NSE) Pipeline Flow...\n")
		if err := scrape_nse.ExecuteAll(ticker, workerCount); err != nil {
			fmt.Fprintf(os.Stderr, "❌ NSE Pipeline failure: %v\n", err)
		}
	} else {
		fmt.Println("\n[engine] ⏭️ Skipping NSE tracking flow per mode flag directive.")
	}

	// ============================================================================
	// PHASE 2: EXECUTE BOMBAY STOCK EXCHANGE (BSE) ENGINE DATA STREAM
	// ============================================================================
	if mode == "bse" || mode == "both" {
		fmt.Printf("\n[engine] 🔵 Triggering Bombay Stock Exchange (BSE) Pipeline Flow...\n")
		if err := scrape_bse.ExecuteAll(ticker, workerCount); err != nil {
			fmt.Fprintf(os.Stderr, "❌ BSE Pipeline failure: %v\n", err)
		}
	} else {
		fmt.Println("\n[engine] ⏭️ Skipping BSE tracking flow per mode flag directive.")
	}

	fmt.Printf("\n=== 🎉 [%s] All Requested Exchange Pipelines Completed in %v ===\n", ticker, time.Since(startTime))
}