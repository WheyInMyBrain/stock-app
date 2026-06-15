package scrape_bse

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
)

// DownloadTask represents a single unit of work for a background BSE worker loop.
type DownloadTask struct {
	URL      string
	SavePath string
	FileName string
}

// downloadFileWorker loops concurrently through job channels and streams assets directly to disk.
func downloadFileWorker(
    client *BSEClient, 
    tasks <-chan DownloadTask, 
    wg *sync.WaitGroup, 
    apiName string, 
    telemetry interface{ WriteLine(string) }, 
    currentStep int, 
    totalSteps int,
) {
    defer wg.Done()

    for task := range tasks {
        // THE CACHE CHECK: Does this file already exist on your disk?
        if _, err := os.Stat(task.SavePath); err == nil {
            fmt.Printf("[bse_worker] ⏭️ Skipped (Already Downloaded): %s\n", task.FileName)
            
            // 🎯 TELEMETRY INSTANT PASS: Signal to the UI that this file is fully synchronized (100.0%)
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", apiName, task.FileName, currentStep, totalSteps))
            }
            continue
        }

        fmt.Printf("[bse_worker] ⏳ Downloading: %s\n", task.FileName)

        req, err := http.NewRequest("GET", task.URL, nil)
        if err != nil {
            fmt.Printf("[bse_worker] ❌ Request fail for %s: %v\n", task.FileName, err)
            continue
        }
        
        // 🛡️ Inject explicit BSE firewall protection metrics
        req.Header.Set("User-Agent", UserAgent)
        req.Header.Set("Origin", Origin)
        req.Header.Set("Referer", Referer)

        resp, err := client.HTTPClient.Do(req)
        if err != nil {
            fmt.Printf("[bse_worker] ❌ Connection error for %s: %v\n", task.FileName, err)
            continue
        }

        // 🛡️ HTTP RESPONSE GUARD: Check status before touching disk filesystem!
        if resp.StatusCode != http.StatusOK {
            fmt.Printf("[bse_worker] ❌ Server rejected %s: HTTP Status Code %d (File missing on BSE servers)\n", task.FileName, resp.StatusCode)
            resp.Body.Close()
            continue 
        }

        // Stream to filesystem block chunks immediately — keeps RAM consumption near 0
        out, err := os.Create(task.SavePath)
        if err != nil {
            fmt.Printf("[bse_worker] ❌ Disk create error for %s: %v\n", task.FileName, err)
            resp.Body.Close()
            continue
        }

        // 🎯 TELEMETRY INJECTION: Wrap the response body stream inside your progress proxy reader.
        // As data chunks flow across network buffers into your filesystem, live stats stream to the socket.
        tracker := &progressTrackingReader{
            Reader:      resp.Body,
            apiName:     apiName,
            filename:    task.FileName,
            totalBytes:  resp.ContentLength,
            currentStep: currentStep,
            totalSteps:  totalSteps,
            telemetry:   telemetry,
        }

        _, err = io.Copy(out, tracker)
        out.Close()
        resp.Body.Close()

        if err != nil {
            fmt.Printf("[bse_worker] ❌ Write processing fail for %s: %v\n", task.FileName, err)
        } else {
            fmt.Printf("[bse_worker] ✅ Finished: %s\n", task.FileName)
            
            // 🎯 TELEMETRY FLUSH: Explicitly issue a clean 100.0% completion confirmation flag down the wire.
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", apiName, task.FileName, currentStep, totalSteps))
            }
        }
    }
}

// buildSaveDirectory builds structural tree target schemas: data/{ticker}/bse_{api_name}
func buildSaveDirectory(symbol, apiName string) (string, error) {
	// 🚀 Added "bse_" prefix cleanly here to ensure absolute isolation!
	exchangeFolder := fmt.Sprintf("bse_%s", apiName)
	baseDir := filepath.Join("data", symbol, exchangeFolder)
	err := os.MkdirAll(baseDir, os.ModePerm)
	return baseDir, err
}