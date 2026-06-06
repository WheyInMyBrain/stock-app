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
    daemonPtr := flag.Bool("daemon", false, "Boot the application as a permanent in-memory scraping service layer")
    fromTimePtr := flag.String("from", "0", "Lower boundary Unix timestamp marker for dynamic real-time chart delta syncs")
    onlyJsonPtr := flag.Bool("only-json", false, "Fetch only primary JSON metadata arrays and completely bypass heavy attachment document parsing loop files")

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
        return
    }

    // ============================================================================
    // LEGACY SINGLE-PASS FALLBACK
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
            } else if strings.HasPrefix(customArg, "-from=") {
                *fromTimePtr = strings.Split(customArg, "=")[1]
            } else if customArg == "-only-json" || customArg == "--only-json" {
                *onlyJsonPtr = true
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
    
    // 🎯 Forward the onlyJson boolean state down into your core strategy execution pipelines
    if mode == "nse" || mode == "both" {
        _ = scrape_nse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir, *fromTimePtr, *onlyJsonPtr)
    }
    if mode == "bse" || mode == "both" {
        _ = scrape_bse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir, *onlyJsonPtr)
    }
    fmt.Printf("\n=== 🎉 [%s] Pipelines Completed in %v ===\n", ticker, time.Since(startTime))
}