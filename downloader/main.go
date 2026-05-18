package main

import (
	"flag"
	"fmt"
	"os"
	"stock-app/downloader/scrape_bse"
	"stock-app/downloader/scrape_nse"
	"time"
)

func main() {
	// 1. Define command-line flags (Flag Name, Default Value, Help Description)
	workerCountPtr := flag.Int("workers", 5, "Number of concurrent background download workers")

	// 2. Custom usage text block if arguments are missing or broken
	flag.Usage = func() {
		fmt.Printf("Usage: go run ./downloader/main.go [options] SYMBOL\n\n")
		fmt.Printf("Options:\n")
		flag.PrintDefaults()
		fmt.Printf("\nExamples:\n")
		fmt.Printf("  go run ./downloader/main.go IMFA               (Uses default 5 workers)\n")
		fmt.Printf("  go run ./downloader/main.go -workers=10 TCS    (Uses custom 10 workers)\n")
	}

	// 3. Parse options from the raw command execution line
	flag.Parse()

	// 4. Any leftover arguments after parsing flags are considered positional arguments (like our SYMBOL)
	args := flag.Args()
	if len(args) < 1 {
		fmt.Println("❌ Error: Missing stock symbol argument.")
		flag.Usage()
		os.Exit(1)
	}

	ticker := args[0]
	workerCount := *workerCountPtr // Dereference the flag pointer to get the internal int value
	startTime := time.Now()

	fmt.Printf("=== 🚀 Starting Multi-Exchange Extraction Engine for: %s (Workers: %d) ===\n", ticker, workerCount)

	// ============================================================================
	// PHASE 1: EXECUTE NATIONAL STOCK EXCHANGE (NSE) ENGINE DATA STREAM
	// ============================================================================
	fmt.Printf("\n[engine] 🟢 Triggering National Stock Exchange (NSE) Pipeline Flow...\n")
	if err := scrape_nse.ExecuteAll(ticker, workerCount); err != nil {
		fmt.Fprintf(os.Stderr, "❌ NSE Pipeline failure: %v\n", err)
		// We continue processing instead of a hard crash, so a failure on one exchange won't kill the other
	}

	// ============================================================================
	// PHASE 2: EXECUTE BOMBAY STOCK EXCHANGE (BSE) ENGINE DATA STREAM
	// ============================================================================
	fmt.Printf("\n[engine] 🔵 Triggering Bombay Stock Exchange (BSE) Pipeline Flow...\n")
	if err := scrape_bse.ExecuteAll(ticker, workerCount); err != nil {
		fmt.Fprintf(os.Stderr, "❌ BSE Pipeline failure: %v\n", err)
	}

	fmt.Printf("\n=== 🎉 [%s] All Exchange Pipelines Completed in %v ===\n", ticker, time.Since(startTime))
}