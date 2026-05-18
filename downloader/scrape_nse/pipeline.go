package scrape_nse

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"
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
	// Mount automated directory path right away: data/{symbol}/{api_name}
	outputDir, err := buildSaveDirectory(symbol, endpoint.Name())
	if err != nil {
		return fmt.Errorf("failed creating directories: %w", err)
	}

	// 🛡️ INTERCEPT CHART STRATEGY: Handle Multi-Timeframe logic dynamically
	if endpoint.Name() == "historical-chart-data" {
		chartAPI, ok := endpoint.(HistoricalChartAPI)
		if !ok {
			return fmt.Errorf("failed type assertion for HistoricalChartAPI")
		}

		directives := chartAPI.ParseMultiTimeframes(symbol)
		for _, dir := range directives {
			targetURL := dir.DownloadURL[12:]
			fmt.Printf("[scrape] 📈 Fetching historical market trend timeline: %s\n", dir.Period)

			req, err := http.NewRequest("GET", targetURL, nil)
			if err != nil {
				return err
			}
			req.Header.Set("User-Agent", UserAgent)
			req.Header.Set("Referer", Referer)
			req.Header.Set("Accept", "*/*")

			resp, err := client.HTTPClient.Do(req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Chart fetch dropped for %s: %v\n", dir.Period, err)
				continue
			}

			if resp.StatusCode != http.StatusOK {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Chart API rejected timeframe %s, status: %d\n", dir.Period, resp.StatusCode)
				resp.Body.Close()
				continue
			}

			chartBytes, err := io.ReadAll(resp.Body)
			resp.Body.Close()
			if err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Read failed for chart %s: %v\n", dir.Period, err)
				continue
			}

			tfPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", dir.Period))
			if err := os.WriteFile(tfPath, chartBytes, 0644); err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Failed saving chart file %s: %v\n", dir.Period, err)
			}

			time.Sleep(150 * time.Millisecond)
		}
		return nil
	}

	// 🛡️ INTERCEPT PEER COMPARISON STRATEGY: Matrix Combination Generator Loop
	if endpoint.Name() == "peer-comparison-matrix" {
		peerAPI, ok := endpoint.(PeerComparisonAPI)
		if !ok {
			return fmt.Errorf("failed type assertion for PeerComparisonAPI")
		}

		combos := peerAPI.GetCombinations(symbol)
		fmt.Printf("[scrape] 📊 Running grid sweeper across %d distinct valuation peer matrix variants...\n", len(combos))

		for _, item := range combos {
			req, err := http.NewRequest("GET", item.URL, nil)
			if err != nil {
				return err
			}
			req.Header.Set("User-Agent", UserAgent)
			req.Header.Set("Referer", Referer)
			req.Header.Set("Accept", "*/*")

			resp, err := client.HTTPClient.Do(req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Peer matrix drop for %s: %v\n", item.FileName, err)
				continue
			}

			if resp.StatusCode != http.StatusOK {
				// Don't log a scary warning for 400 or 404 since companies don't sit in all indices simultaneously
				resp.Body.Close()
				continue
			}

			peerBytes, err := io.ReadAll(resp.Body)
			resp.Body.Close()
			if err != nil {
				continue
			}

			// Write out unique files directly (e.g. Industry_2025-12.json, Index_NIFTY_MICROCAP_250_2025-03.json)
			matrixPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", item.FileName))
			if err := os.WriteFile(matrixPath, peerBytes, 0644); err != nil {
				fmt.Fprintf(os.Stderr, "[scrape] ❌ Failed writing peer matrix %s: %v\n", item.FileName, err)
			}

			// Polite pacing delay to protect session health boundaries
			time.Sleep(150 * time.Millisecond)
		}
		return nil
	}

	// ============================================================================
	// STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL OTHER ENDPOINTS
	// ============================================================================
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

	rawBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed reading body bytes: %w", err)
	}

	metaJSONPath := filepath.Join(outputDir, "endpoint-metadata.json")
	fmt.Printf("[scrape] 📝 Archiving raw response array payload to: %s\n", metaJSONPath)
	if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
		fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Warning: Failed saving metadata JSON file: %v\n", err)
	}

	bodyReader := bytes.NewReader(rawBytes)
	records, err := endpoint.ParseResponse(bodyReader)
	if err != nil {
		return fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
	}

	fmt.Printf("[scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)

	if len(records) == 0 {
		return nil
	}

	tasksChan := make(chan DownloadTask, len(records))
	var wg sync.WaitGroup

	for w := 1; w <= workerCount; w++ {
		wg.Add(1)
		go downloadFileWorker(client, tasksChan, &wg)
	}

	for _, row := range records {
		if row.DownloadURL == "" || row.DownloadURL == "-" || len(row.DownloadURL) < 8 {
			fmt.Printf("[scrape] ⚠️ Skipping entry '%s': Invalid or empty download URL string.\n", row.Period)
			continue
		}

		if row.DownloadURL[:4] != "http" {
			fmt.Printf("[scrape] ⚠️ Skipping entry '%s': Unsupported url prefix: %s\n", row.Period, row.DownloadURL)
			continue
		}

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