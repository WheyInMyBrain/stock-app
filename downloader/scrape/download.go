package scrape

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
)

type DownloadTask struct {
	URL      string
	SavePath string
	FileName string
}

// downloadFileWorker loops concurrently through job channels and streams assets directly to disk.
func downloadFileWorker(client *NSEClient, tasks <-chan DownloadTask, wg *sync.WaitGroup) {
	defer wg.Done()

	for task := range tasks {
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

		if resp.StatusCode != http.StatusOK {
			fmt.Printf("[worker] ❌ Server rejected %s, status: %d\n", task.FileName, resp.StatusCode)
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

		_, err = io.Copy(out, resp.Body)
		out.Close()
		resp.Body.Close()

		if err != nil {
			fmt.Printf("[worker] ❌ Write processing fail for %s: %v\n", task.FileName, err)
		} else {
			fmt.Printf("[worker] ✅ Finished: %s\n", task.FileName)
		}
	}
}

// buildSaveDirectory builds structural tree target schemas: ../data/{ticker}/{api_name}
func buildSaveDirectory(symbol, apiName string) (string, error) {
	baseDir := filepath.Join("..", "data", symbol, apiName)
	err := os.MkdirAll(baseDir, os.ModePerm)
	return baseDir, err
}