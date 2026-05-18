package scrape

import (
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"sync"
)

// ExecuteAll is the sole gateway for main.go.
// It fetches all registered strategies from endpoints.go and runs them sequentially.
func ExecuteAll(symbol string, workerCount int) error {
	// Initialize your session & cookie handshake once here
	client, err := NewNSEClient()
	if err != nil {
		return fmt.Errorf("session handshake failed: %w", err)
	}

	// Dynamic endpoint array loaded from endpoints.go
	endpoints := GetAllEndpoints()

	for _, endpoint := range endpoints {
		fmt.Printf("\n[scrape] 🌀 Running downloader for target endpoint: %s\n", endpoint.Name())

		// Execute each strategy using the shared, authenticated client
		if err := executeStrategy(client, symbol, endpoint, workerCount); err != nil {
			fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Error running pipeline %s: %v\n", endpoint.Name(), err)
			// Continue to the next API even if one fails
		}
	}

	return nil
}

// executeStrategy is unexported (private) to keep the package API surface tiny.
func executeStrategy(client *NSEClient, symbol string, endpoint FilingsEndpoint, workerCount int) error {
	// 1. Fetch records dynamically using the strategy blueprint
	apiURL := endpoint.BuildURL(symbol)
	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return err
	}
	req.Header.Set("User-Agent", UserAgent)
	req.Header.Set("Referer", Referer)
	req.Header.Set("Accept", "*/*")

	resp, err := client.HTTPClient.Do(req)
	if err != nil {
		return fmt.Errorf("failed fetching data from %s: %w", endpoint.Name(), err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("API %s rejected request with status: %d", endpoint.Name(), resp.StatusCode)
	}

	records, err := endpoint.ParseResponse(resp.Body)
	if err != nil {
		return fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
	}

	fmt.Printf("[scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)
	if len(records) == 0 {
		return nil
	}

	// 2. Mount automated directory path: ../data/{symbol}/{api_name}
	outputDir, err := buildSaveDirectory(symbol, endpoint.Name())
	if err != nil {
		return fmt.Errorf("failed creating directories: %w", err)
	}

	// 3. Spin up concurrent lanes
	tasksChan := make(chan DownloadTask, len(records))
	var wg sync.WaitGroup

	for w := 1; w <= workerCount; w++ {
		wg.Add(1)
		go downloadFileWorker(client, tasksChan, &wg)
	}

	// 4. Feed records into jobs queue channel array
	for _, row := range records {
		// CHECK: Is this a raw text data dump instead of an external file download link?
		if len(row.DownloadURL) > 10 && row.DownloadURL[:10] == "DATA_DUMP:" {
			// Strip our custom prefix keyword to get back the pure JSON text string
			pureJSONText := row.DownloadURL[10:]
			
			jsonPath := filepath.Join(outputDir, row.Period+".json")
			fmt.Printf("[scrape] 💾 Saving structural tabular data to: %s.json\n", row.Period)
			
			// Write the data payload directly onto your disk storage
			if err := os.WriteFile(jsonPath, []byte(pureJSONText), 0644); err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Failed to write JSON data file: %v\n", err)
			}
			continue // Skip adding it to the file downloader worker threads!
		}

		// --- Standard external file downloading logic continues safely below ---
		ext := filepath.Ext(row.DownloadURL)
		if ext == "" {
			ext = ".xml" 
		}
		
		localName := fmt.Sprintf("%s%s", row.Period, ext)
		fullDiskPath := filepath.Join(outputDir, localName)

		tasksChan <- DownloadTask{
			URL:      row.DownloadURL,
			SavePath: fullDiskPath,
			FileName: localName,
		}
	}
	close(tasksChan)

	wg.Wait()
	return nil
}