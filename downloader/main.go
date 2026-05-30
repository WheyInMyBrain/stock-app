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
    // 1. Define command-line flags
    workerCountPtr := flag.Int("workers", 5, "Number of concurrent background download workers")
    modePtr := flag.String("mode", "both", "Target exchange execution strategy: 'nse', 'bse', or 'both'")
    apiPtr := flag.String("api", "", "Name of the single target endpoint to execute (e.g., 'historical-chart-data')")
    // Explicit target directory pointer flag mapping
    dataDirPtr := flag.String("data-dir", "", "Absolute path to the global unified data directory storage core")

    // 2. Custom usage text block if arguments are missing or broken
    flag.Usage = func() {
        fmt.Printf("Usage: go run ./downloader/main.go [options] SYMBOL\n\n")
        fmt.Printf("Options:\n")
        flag.PrintDefaults()
        fmt.Printf("\nExamples:\n")
        fmt.Printf("  go run ./downloader/main.go IMFA                                 (Scrapes both completely)\n")
        fmt.Printf("  go run ./downloader/main.go -mode=nse -api=symbol-core-data -data-dir=/absolute/path IMFA\n")
    }

    // 3. Parse options from the raw command execution line
    flag.Parse()

    // 4. Extract target positional symbol
    args := flag.Args()
    var ticker string
    if len(args) >= 1 {
        // Assume the first clean non-flag argument is our ticker (e.g., "IMFA")
        ticker = args[0]
        
        // Manual scan for trailing misplaced flag strings like "-mode=bse", "-api=xxx", or "-data-dir=xxx"
        for _, customArg := range args[1:] {
            if strings.HasPrefix(customArg, "-mode=") {
                *modePtr = strings.Split(customArg, "=")[1]
            } else if strings.HasPrefix(customArg, "-api=") {
                *apiPtr = strings.Split(customArg, "=")[1]
            } else if strings.HasPrefix(customArg, "-data-dir=") {
                // 🎯 FIXED: Cleanly intercepts inline trailing path arguments passed by the layout core
                *dataDirPtr = strings.Split(customArg, "=")[1]
            } else if customArg == "-mode" || customArg == "-api" || customArg == "-data-dir" {
                fmt.Printf("❌ Syntax Error: Misplaced flag order for '%s'. Use clean syntax format: -flag=value\n", customArg)
            }
        }
    } else {
        fmt.Println("❌ Error: Missing stock symbol argument.")
        flag.Usage()
        os.Exit(1)
    }

    workerCount := *workerCountPtr
    mode := strings.ToLower(strings.TrimSpace(*modePtr))
    targetApi := strings.TrimSpace(*apiPtr) 
    globalDataDir := strings.TrimSpace(*dataDirPtr) // 🎯 FIXED: Read parsed filesystem destination string block
    startTime := time.Now()

    // Validation guard for the execution mode flag
    if mode != "nse" && mode != "bse" && mode != "both" {
        fmt.Printf("❌ Error: Invalid execution mode '%s'. Choose between 'nse', 'bse', or 'both'.\n", mode)
        os.Exit(1)
    }

    fmt.Printf("=== 🚀 Starting Multi-Exchange Extraction Engine for: %s (Workers: %d, Mode: %s, API: %s) ===\n", ticker, workerCount, mode, targetApi)
    if globalDataDir != "" {
        fmt.Printf("[engine] 📍 Explicit Global Data Directory Target: %s\n", globalDataDir)
    }

    // ============================================================================
    // PHASE 1: EXECUTE NATIONAL STOCK EXCHANGE (NSE) ENGINE DATA STREAM
    // ============================================================================
    if mode == "nse" || mode == "both" {
        fmt.Printf("\n[engine] 🟢 Triggering National Stock Exchange (NSE) Pipeline Flow...\n")
        // 🎯 FIXED: Passed globalDataDir downstream straight into the NSE runner context loop
        if err := scrape_nse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir); err != nil {
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
        // 🎯 FIXED: Passed globalDataDir downstream straight into the BSE runner context loop
        if err := scrape_bse.ExecuteAll(ticker, workerCount, targetApi, globalDataDir); err != nil {
            fmt.Fprintf(os.Stderr, "❌ BSE Pipeline failure: %v\n", err)
        }
    } else {
        fmt.Println("\n[engine] ⏭️ Skipping BSE tracking flow per mode flag directive.")
    }

    fmt.Printf("\n=== 🎉 [%s] All Requested Exchange Pipelines Completed in %v ===\n", ticker, time.Since(startTime))
}