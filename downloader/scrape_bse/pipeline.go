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
	"strings"
)

// ExecuteWithWarmClient serves as the blazing-fast daemon gateway for the BSE stack.
// It skips creating fresh client structures, reusing the pre-warmed connection pointer pool.
func ExecuteWithWarmClient(client *BSEClient, symbol string, workerCount int, targetApi string, globalDataDir string) (string, error) {
    // 1. Resolve the alphabetic ticker symbol ("IMFA") into its BSE numeric code ("533047")
    fmt.Printf("[bse_scrape] 🔍 Performing smart search lookup for ticker token: %s...\n", symbol)
    scripCode, err := GetScripCode(client, symbol)
    if err != nil {
        return "", fmt.Errorf("BSE identifier mapping failed: %w", err)
    }
    fmt.Printf("[bse_scrape] 🎯 Successfully mapped %s ----> BSE Scrip Code: %s\n", symbol, scripCode)

    // 2. Dynamic endpoint array loaded from endpoints.go
    endpoints := GetAllEndpoints()
    var capturedJSON string

    for _, endpoint := range endpoints {
        // If a specific API is requested, bypass everything that doesn't match its endpoint name!
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        fmt.Printf("\n[bse_scrape] 🌀 Running downloader for target endpoint: %s\n", endpoint.Name())

        // Execute each strategy using the shared, authenticated client and resolved scripCode
        rawBytes, err := executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir)
        if err != nil {
            fmt.Fprintf(os.Stderr, "[bse_scrape] ⚠️ Error running pipeline %s: %v\n", endpoint.Name(), err)
            return "", err
        }

        // If the execution pulled valid network data bytes, cast them to a string reference frame
        if len(rawBytes) > 0 {
            capturedJSON = string(rawBytes)
        }
    }

    return capturedJSON, nil
}

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

// ExecuteAll serves as the single execution gateway from main.go for the BSE legacy pipeline network.
func ExecuteAll(symbol string, workerCount int, targetApi string, globalDataDir string) error {
	client, err := NewBSEClient()
	if err != nil {
		return fmt.Errorf("BSE session initialization failed: %w", err)
	}

	fmt.Printf("[bse_scrape] 🔍 Performing smart search lookup for ticker token: %s...\n", symbol)
	scripCode, err := GetScripCode(client, symbol)
	if err != nil {
		return fmt.Errorf("BSE identifier mapping failed: %w", err)
	}
	fmt.Printf("[bse_scrape] 🎯 Successfully mapped %s ----> BSE Scrip Code: %s\n", symbol, scripCode)

	endpoints := GetAllEndpoints()
	if len(endpoints) == 0 {
		fmt.Println("[bse_scrape] ℹ️ No active BSE endpoint strategies registered yet.")
		return nil
	}

	for _, endpoint := range endpoints {
		if targetApi != "" && endpoint.Name() != targetApi {
			continue
		}

		fmt.Printf("\n[bse_scrape] 🌀 Running downloader for target endpoint: %s\n", endpoint.Name())

		// 🎯 FIXED: Discard the streaming byte outputs to match updated 2-value return signature
		_, _ = executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir)
	}

	return nil
}

// executeStrategy maps out the processing loop safely.
// Updated signature syntax schema to return ([]byte, error) natively up the tracking context chain.
func executeStrategy(client *BSEClient, symbol, scripCode string, endpoint BSEFilingsEndpoint, workerCount int, globalDataDir string) ([]byte, error) {
    var outputDir string

    // If Rust provides an explicit global data directory path, anchor it instantly!
    // Otherwise, drop down to the bulletproof absolute fallback path calculator for raw terminal runs.
    if globalDataDir != "" {
        outputDir = filepath.Join(globalDataDir, symbol, "bse_"+endpoint.Name())
        if err := os.MkdirAll(outputDir, 0755); err != nil {
            return nil, fmt.Errorf("failed creating explicit global BSE target directory: %w", err)
        }
    } else {
        // Mount automated directory path right away: data/{symbol}/bse_{api_name}
        baseDir, err := buildSaveDirectory(symbol, endpoint.Name())
        if err != nil {
            return nil, fmt.Errorf("failed creating directories: %w", err)
        }

        // 🎯 DEFINITIVE ABSOLUTE PATH RESOLUTION FOR BSE
        absPath, err := filepath.Abs(baseDir)
        if err != nil {
            return nil, fmt.Errorf("failed to compute absolute path context: %w", err)
        }

        if strings.Contains(absPath, filepath.Join("downloader", "data")) {
            parts := strings.Split(absPath, filepath.Join("downloader", "data"))
            outputDir = filepath.Join(parts[0], "data", symbol, "bse_"+endpoint.Name())
            if err := os.MkdirAll(outputDir, 0755); err != nil {
                return nil, fmt.Errorf("failed generating unified parent directory mapping: %w", err)
            }
        } else {
            outputDir = absPath
        }
    }

    // ============================================================================
    // 🛡️ INTERCEPT DEALS STRATEGY: Handle Bulk (type=1) and Block (type=2) dynamics sequentially
    // ============================================================================
    if endpoint.Name() == "bulk-block-deals" {
        dealsAPI, ok := endpoint.(BSEBulkBlockDealsAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for BSEBulkBlockDealsAPI")
        }

        directives := dealsAPI.ParseDeals(scripCode)
        var lastDealsBytes []byte

        for _, dir := range directives {
            targetURL := dir.DownloadURL[15:]
            fmt.Printf("[bse_scrape] 📊 Fetching institutional market transaction layer: %s\n", dir.Period)

            req, err := http.NewRequest("GET", targetURL, nil)
            if err != nil {
                return nil, err
            }
            req.Header.Set("User-Agent", UserAgent)
            req.Header.Set("Accept", "application/json, text/plain, */*")
            req.Header.Set("Origin", Origin)
            req.Header.Set("Referer", Referer)

            resp, err := client.HTTPClient.Do(req)
            if err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Connection error dropped deal fetch for %s: %v\n", dir.Period, err)
                continue
            }

            if resp.StatusCode != http.StatusOK {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ BSE API rejected deal entry %s, status code: %d\n", dir.Period, resp.StatusCode)
                resp.Body.Close()
                continue
            }

            dealBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Read failed for transaction stream row %s: %v\n", dir.Period, err)
                continue
            }

            dealPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", dir.Period))
            if err := os.WriteFile(dealPath, dealBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Failed writing transaction file to disk %s: %v\n", dir.Period, err)
            }

            lastDealsBytes = dealBytes
            time.Sleep(150 * time.Millisecond)
        }
        return lastDealsBytes, nil 
    }

    // ============================================================================
    // 🛡️ INTERCEPT CHART STRATEGY: Handle History chart timewise dynamics sequentially
    // ============================================================================
    if endpoint.Name() == "historical-chart-data" {
        chartAPI, ok := endpoint.(interface {
            ParseMultiHorizons(scripCode string) []UniversalRecord
            ProcessAndNormalize(outputDir, period string, rawBytes []byte) error
        })
        if !ok {
            return nil, fmt.Errorf("failed interface contract lookup for historical-chart-data transformer")
        }

        directives := chartAPI.ParseMultiHorizons(scripCode)
        var lastChartBytes []byte

        for _, dir := range directives {
            targetURL := dir.DownloadURL[17:] 
            fmt.Printf("[bse_scrape] 📈 Processing and transforming tracking metrics: %s\n", dir.Period)

            req, err := http.NewRequest("GET", targetURL, nil)
            if err != nil {
                return nil, err
            }
            req.Header.Set("User-Agent", UserAgent)
            req.Header.Set("Accept", "application/json, text/plain, */*")
            req.Header.Set("Origin", Origin)
            req.Header.Set("Referer", Referer)

            resp, err := client.HTTPClient.Do(req)
            if err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Connection failure for chart horizon %s: %v\n", dir.Period, err)
                continue
            }

            chartBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Read failed for chart stream %s: %v\n", dir.Period, err)
                continue
            }

            if err := chartAPI.ProcessAndNormalize(outputDir, dir.Period, chartBytes); err != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ❌ Transformation loop failed for %s: %v\n", dir.Period, err)
            }

            lastChartBytes = chartBytes
            time.Sleep(150 * time.Millisecond)
        }
        return lastChartBytes, nil 
    }

    // ============================================================================
    // STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL BSE ENDPOINTS
    // ============================================================================
    apiURL := endpoint.BuildURL(scripCode)
    req, err := http.NewRequest("GET", apiURL, nil)
    if err != nil {
        return nil, err
    }
    
    req.Header.Set("User-Agent", UserAgent)
    req.Header.Set("Accept", "application/json, text/plain, */*")
    req.Header.Set("Origin", Origin)
    req.Header.Set("Referer", Referer)

    resp, err := client.HTTPClient.Do(req)
    if err != nil {
        return nil, fmt.Errorf("failed fetching data from %s: %w", endpoint.Name(), err)
    }
    defer resp.Body.Close()

    if resp.StatusCode != http.StatusOK {
        return nil, fmt.Errorf("API %s rejected request with status: %d", endpoint.Name(), resp.StatusCode)
    }

    rawBytes, err := io.ReadAll(resp.Body)
    if err != nil {
        return nil, fmt.Errorf("failed reading body bytes: %w", err)
    }

    metaJSONPath := filepath.Join(outputDir, "endpoint-metadata.json")
    fmt.Printf("[bse_scrape] 📝 Archiving raw response array payload to: %s\n", metaJSONPath)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        fmt.Fprintf(os.Stderr, "[bse_scrape] ⚠️ Warning: Failed saving metadata JSON file: %v\n", err)
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    fmt.Printf("[bse_scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)

    if len(records) == 0 {
        return rawBytes, nil 
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
    return rawBytes, nil
}