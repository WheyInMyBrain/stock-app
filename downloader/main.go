package main

import (
	"fmt"
	"os"
	"stock-app/downloader/scrape"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("❌ Error: Missing stock symbol argument.")
		fmt.Println("Usage: go run main.go [SYMBOL]")
		os.Exit(1)
	}

	ticker := os.Args[1]
	workerCount := 4
	startTime := time.Now()

	fmt.Printf("=== 🚀 Starting Multi-Pipeline Engine for: %s ===\n", ticker)

	// One single call to fire off every registered endpoint internally!
	if err := scrape.ExecuteAll(ticker, workerCount); err != nil {
		fmt.Fprintf(os.Stderr, "❌ Critical engine failure: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("\n=== 🎉 [%s] All Pipelines Finished in %v ===\n", ticker, time.Since(startTime))
}