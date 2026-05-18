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
func downloadFileWorker(client *BSEClient, tasks <-chan DownloadTask, wg *sync.WaitGroup) {
	defer wg.Done()

	for task := range tasks {
		// THE CACHE CHECK: Does this file already exist on your disk?
		if _, err := os.Stat(task.SavePath); err == nil {
			fmt.Printf("[bse_worker] ⏭️ Skipped (Already Downloaded): %s\n", task.FileName)
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
			continue // Skip creating an empty local file, proceed to next queue element
		}

		// Stream to filesystem block chunks immediately — keeps RAM consumption near 0
		out, err := os.Create(task.SavePath)
		if err != nil {
			fmt.Printf("[bse_worker] ❌ Disk create error for %s: %v\n", task.FileName, err)
			resp.Body.Close()
			continue
		}

		_, err = io.Copy(out, resp.Body)
		out.Close()
		resp.Body.Close()

		if err != nil {
			fmt.Printf("[bse_worker] ❌ Write processing fail for %s: %v\n", task.FileName, err)
		} else {
			fmt.Printf("[bse_worker] ✅ Finished: %s\n", task.FileName)
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