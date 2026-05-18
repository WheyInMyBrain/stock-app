package scrape_bse

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

// UniversalRecord standardizes asset data rows decoded by individual BSE endpoints.
type UniversalRecord struct {
	Period      string // Naming token (e.g., "Quarter_Dec_2025")
	DownloadURL string // Path to target document file download or data action
}

// BSEFilingsEndpoint establishes structural blueprints for all future BSE parsing strategies.
type BSEFilingsEndpoint interface {
	Name() string
	BuildURL(scripCode string) string
	ParseResponse(body io.Reader) ([]UniversalRecord, error)
}

// ExecuteAll serves as the single execution gateway from main.go for the BSE pipeline network.
func ExecuteAll(symbol string, workerCount int) error {
	// 1. Initialize your session & cookie handshake once here
	client, err := NewBSEClient()
	if err != nil {
		return fmt.Errorf("BSE session initialization failed: %w", err)
	}

	// 2. Resolve the alphabetic ticker symbol ("IMFA") into its BSE numeric code ("533047")
	fmt.Printf("[bse_scrape] 🔍 Performing smart search lookup for ticker token: %s...\n", symbol)
	scripCode, err := GetScripCode(client, symbol)
	if err != nil {
		return fmt.Errorf("BSE identifier mapping failed: %w", err)
	}
	fmt.Printf("[bse_scrape] 🎯 Successfully mapped %s ----> BSE Scrip Code: %s\n", symbol, scripCode)

	// 3. Dynamic endpoint array loaded from endpoints.go (We will build endpoints.go next)
	endpoints := GetAllEndpoints()
	if len(endpoints) == 0 {
		fmt.Println("[bse_scrape] ℹ️ No active BSE endpoint strategies registered yet.")
		return nil
	}

	for _, endpoint := range endpoints {
		fmt.Printf("\n[bse_scrape] 🌀 Running downloader for target endpoint: %s\n", endpoint.Name())

		// Execute each strategy using the shared, authenticated client and resolved scripCode
		if err := executeStrategy(client, symbol, scripCode, endpoint, workerCount); err != nil {
			fmt.Fprintf(os.Stderr, "[bse_scrape] ⚠️ Error running pipeline %s: %v\n", endpoint.Name(), err)
			// Continue to the next API even if one fails
		}
	}

	return nil
}

// executeStrategy maps out the processing loop safely.
func executeStrategy(client *BSEClient, symbol, scripCode string, endpoint BSEFilingsEndpoint, workerCount int) error {
	// Mount automated directory path right away: data/{symbol}/bse_{api_name}
	outputDir, err := buildSaveDirectory(symbol, endpoint.Name())
	if err != nil {
		return fmt.Errorf("failed creating directories: %w", err)
	}

	// 🛡️ INTERCEPT CHART STRATEGY: Handle BSE Multi-Timeframe logic dynamically
	if endpoint.Name() == "historical-chart-data" {
		chartAPI, ok := endpoint.(BSEHistoricalChartAPI)
		if !ok {
			return fmt.Errorf("failed type assertion for BSEHistoricalChartAPI")
		}

		// Grab the 4 custom data collection directives
		directives := chartAPI.ParseMultiHorizons(scripCode)

		for _, dir := range directives {
			// Strip out our action prefix token ("BSE_CHART_FETCH:") to isolate the target URL path string
			targetURL := dir.DownloadURL[17:]
			fmt.Printf("[bse_scrape] 📈 Fetching historical market chart horizon timeline: %s\n", dir.Period)

			req, err := http.NewRequest("GET", targetURL, nil)
			if err != nil {
				return err
			}
			req.Header.Set("User-Agent", UserAgent)
			req.Header.Set("Accept", "application/json, text/plain, */*")
			req.Header.Set("Origin", Origin)
			req.Header.Set("Referer", Referer)

			resp, err := client.HTTPClient.Do(req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Chart tracking drop for horizon %s: %v\n", dir.Period, err)
				continue
			}

			if resp.StatusCode != http.StatusOK {
				fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ BSE Chart API rejected timeframe %s, status code: %d\n", dir.Period, resp.StatusCode)
				resp.Body.Close()
				continue
			}

			chartBytes, err := io.ReadAll(resp.Body)
			resp.Body.Close()
			if err != nil {
				fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Failed reading chart raw bytes stream for horizon %s: %v\n", dir.Period, err)
				continue
			}

			// Save data points directly as clean standalone timeframe logs!
			tfPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", dir.Period))
			if err := os.WriteFile(tfPath, chartBytes, 0644); err != nil {
				fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Failed writing local chart log %s: %v\n", dir.Period, err)
			}

			// Pacing buffer delay (150ms) to ensure continuous backend access integrity
			time.Sleep(150 * time.Millisecond)
		}
		return nil // Chart compilation sequence fully handled! Skip past standard workflow.
	}

	// ============================================================================
	// STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL BSE ENDPOINTS
	// ============================================================================
	apiURL := endpoint.BuildURL(scripCode)
	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return err
	}
	
	req.Header.Set("User-Agent", UserAgent)
	req.Header.Set("Accept", "application/json, text/plain, */*")
	req.Header.Set("Origin", Origin)
	req.Header.Set("Referer", Referer)

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
	fmt.Printf("[bse_scrape] 📝 Archiving raw response array payload to: %s\n", metaJSONPath)
	if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
		fmt.Fprintf(os.Stderr, "[bse_scrape] ⚠️ Warning: Failed saving metadata JSON file: %v\n", err)
	}

	bodyReader := bytes.NewReader(rawBytes)
	records, err := endpoint.ParseResponse(bodyReader)
	if err != nil {
		return fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
	}

	fmt.Printf("[bse_scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)

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
			fmt.Printf("[bse_scrape] ⚠️ Skipping entry '%s': Invalid or empty download URL string.\n", row.Period)
			continue
		}

		if row.DownloadURL[:4] != "http" {
			fmt.Printf("[bse_scrape] ⚠️ Skipping entry '%s': Unsupported url prefix: %s\n", row.Period, row.DownloadURL)
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