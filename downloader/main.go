package main

import (
	"flag"
	"fmt"
	"os"
	"downloader/scrape_bse"
	"downloader/scrape_nse"
	"strings"
	"time"
)

func main() {
	// 1. Keep your existing command-line flags perfectly intact
	workerCountPtr := flag.Int("workers", 5, "Number of concurrent background download workers")
	modePtr := flag.String("mode", "both", "Target exchange execution strategy: 'nse', 'bse', or 'both'")
	apiPtr := flag.String("api", "", "Name of the single target endpoint to execute (e.g., 'historical-chart-data')")
	dataDirPtr := flag.String("data-dir", "", "Absolute path to the global unified data directory storage core")

	// 🎯 ADD THE PERSISTENT DAEMON TOGGLE FLAG
	daemonPtr := flag.Bool("daemon", false, "Boot the application as a permanent in-memory scraping service layer")

	flag.Usage = func() {
		fmt.Printf("Usage: go run ./downloader/main.go [options] SYMBOL\n\n")
		fmt.Printf("Options:\n")
		flag.PrintDefaults()
	}

	flag.Parse()

	globalDataDir := strings.TrimSpace(*dataDirPtr)
	workerCount := *workerCountPtr

	// ============================================================================
	// 🎯 DECOUPLING GUARD: ROUTE STAGE STRAIGHT TO DAEMON.GO IF TOGGLED
	// ============================================================================
	if *daemonPtr {
		RunPersistentDaemonMode(workerCount, globalDataDir)
		return // Exits here so the application hooks into the endless RAM thread process loop
	}

	// ============================================================================
	// LEGACY SINGLE-PASS FALLBACK
	// (Your exact original CLI execution flow remains untouched below)
	// ============================================================================
	args := flag.Args()
	var ticker string
	if len(args) >= 1 {
		ticker = args[0]
		for _, customArg := range args[1:] {
			if strings.HasPrefix(customArg, "-mode=") {
				*modePtr = strings.Split(customArg, "=")[1]
			} else if strings.HasPrefix(customArg, "-api=") {
				*apiPtr = strings.Split(customArg, "=")[1]
			} else if strings.HasPrefix(customArg, "-data-dir=") {
				*dataDirPtr = strings.Split(customArg, "=")[1]
			}
		}
	} else {
		fmt.Println("❌ Error: Missing stock symbol argument.")
		flag.Usage()
		os.Exit(1)
	}

	mode := strings.ToLower(strings.TrimSpace(*modePtr))
	targetApi := strings.TrimSpace(*apiPtr) 
	startTime := time.Now()

	fmt.Printf("=== 🚀 Starting Multi-Exchange Extraction Engine for: %s ===\n", ticker)
	if mode == "nse" || mode == "both" {
		_ = scrape_nse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir)
	}
	if mode == "bse" || mode == "both" {
		_ = scrape_bse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir)
	}
	fmt.Printf("\n=== 🎉 [%s] Pipelines Completed in %v ===\n", ticker, time.Since(startTime))
}