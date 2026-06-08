package scrape_nse

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
)

// DownloadTask represents a single unit of work for a background worker loop.
type DownloadTask struct {
	URL      string
	SavePath string
	FileName string
}

// downloadFileWorker loops concurrently through job channels and streams assets directly to disk.
func downloadFileWorker(
    client *NSEClient, 
    tasks <-chan DownloadTask, 
    wg *sync.WaitGroup, 
    apiName string, 
    telemetry interface{ WriteLine(string) }, 
    currentStep int, 
    totalSteps int,
) {
    defer wg.Done()

    for task := range tasks {
        if _, err := os.Stat(task.SavePath); err == nil {
            fmt.Printf("[worker] ⏭️ Skipped (Already Downloaded): %s\n", task.FileName)
            
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", apiName, task.FileName, currentStep, totalSteps))
            }
            continue
        }

        fmt.Printf("[worker] ⏳ Downloading: %s\n", task.FileName)

        req, err := http.NewRequest("GET", task.URL, nil)
        if err != nil {
            fmt.Printf("[worker] ❌ Request fail for %s: %v\n", task.FileName, err)
            continue
        }
        req.Header.Set("User-Agent", UserAgent)
        req.Header.Set("Referer", Referer)

        resp, err := client.HTTPClient.Do(req)
        if err != nil {
            fmt.Printf("[worker] ❌ Connection error for %s: %v\n", task.FileName, err)
            continue
        }

        // 🛡️ HTTP RESPONSE GUARD: Check status before touch disk file system!
        if resp.StatusCode != http.StatusOK {
            fmt.Printf("[worker] ❌ Server rejected %s: HTTP Status Code %d (File missing on NSE servers)\n", task.FileName, resp.StatusCode)
            resp.Body.Close()
            continue 
        }

        // Stream to filesystem block chunks immediately — keeps RAM consumption near 0
        out, err := os.Create(task.SavePath)
        if err != nil {
            fmt.Printf("[worker] ❌ Disk create error for %s: %v\n", task.FileName, err)
            resp.Body.Close()
            continue
        }

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
            fmt.Printf("[worker] ❌ Write processing fail for %s: %v\n", task.FileName, err)
        } else {
            fmt.Printf("[worker] ✅ Finished: %s\n", task.FileName)
            
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", apiName, task.FileName, currentStep, totalSteps))
            }
        }
    }
}

// buildSaveDirectory builds structural tree target schemas: data/{ticker}/nse_{api_name}
func buildSaveDirectory(symbol, apiName string) (string, error) {
	// 🚀 Added "nse_" prefix cleanly here!
	exchangeFolder := fmt.Sprintf("nse_%s", apiName)
	baseDir := filepath.Join("data", symbol, exchangeFolder)
	err := os.MkdirAll(baseDir, os.ModePerm)
	return baseDir, err
}