package scrape

import (
	"bytes"
	"fmt"
	"io"
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

	// 💾 READ THE RAW BYTES FIRST: So we can write the clean JSON file to disk
	rawBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed reading body bytes: %w", err)
	}

	// Mount automated directory path: ../data/{symbol}/{api_name}
	outputDir, err := buildSaveDirectory(symbol, endpoint.Name())
	if err != nil {
		return fmt.Errorf("failed creating directories: %w", err)
	}

	// 📝 Save the raw JSON data file exactly as NSE returned it
	metaJSONPath := filepath.Join(outputDir, "endpoint-metadata.json")
	fmt.Printf("[scrape] 📝 Archiving raw response array payload to: %s\n", metaJSONPath)
	if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
		fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Warning: Failed saving metadata JSON file: %v\n", err)
	}

	// Refeed the read bytes into a new reader so our parser strategies can decode it safely
	bodyReader := bytes.NewReader(rawBytes)
	records, err := endpoint.ParseResponse(bodyReader)
	if err != nil {
		return fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
	}

	fmt.Printf("[scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)
	
	// If the strategy doesn't have any files to download (like Corporate Actions), wrap up cleanly here
	if len(records) == 0 {
		return nil
	}

	// 2. Spin up concurrent worker pool lanes
	tasksChan := make(chan DownloadTask, len(records))
	var wg sync.WaitGroup

	for w := 1; w <= workerCount; w++ {
		wg.Add(1)
		go downloadFileWorker(client, tasksChan, &wg)
	}

	// 3. Feed records into jobs queue channel array
	for _, row := range records {
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