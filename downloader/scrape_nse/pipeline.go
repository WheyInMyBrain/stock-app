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
	"strings"
)

// ExecuteWithWarmClient now returns the raw JSON payload as a string alongside any network errors
func ExecuteWithWarmClient(client *NSEClient, symbol string, workerCount int, targetApi string, globalDataDir string) (string, error) {
    endpoints := GetAllEndpoints()
    var capturedJSON string

    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        // 🎯 CAPTURE BOTH THE RAW BYTES AND ERROR FROM THE STRATEGY PASS
        rawBytes, err := executeStrategy(client, symbol, endpoint, workerCount, globalDataDir)
        if err != nil {
            fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Error running warm pipeline %s: %v\n", endpoint.Name(), err)
            return "", err
        }

        // If the execution pulled valid network data bytes, cast them to a string reference frame
        if len(rawBytes) > 0 {
            capturedJSON = string(rawBytes)
        }
    }

    return capturedJSON, nil
}

// ExecuteAll is the sole gateway for main.go.
// It fetches all registered strategies from endpoints.go and runs them sequentially.
func ExecuteAll(symbol string, workerCount int, targetApi string, globalDataDir string) error {
    // Initialize your session & cookie handshake once here
    client, err := NewNSEClient()
    if err != nil {
        return fmt.Errorf("session handshake failed: %w", err)
    }

    // Dynamic endpoint array loaded from endpoints.go
    endpoints := GetAllEndpoints()

    for _, endpoint := range endpoints {
		if targetApi != "" && endpoint.Name() != targetApi {
			continue
		}

		fmt.Printf("\n[scrape] 🌀 Running downloader for target endpoint: %s\n", endpoint.Name())

		// 🎯 FIXED: Change this line to expect 2 return variables instead of 1
		_, err := executeStrategy(client, symbol, endpoint, workerCount, globalDataDir)
		if err != nil {
			fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Error running warm pipeline %s: %v\n", endpoint.Name(), err)
		}
	}

    return nil
}

// executeStrategy is unexported (private) to keep the package API surface tiny.
// Added globalDataDir to signature to handle explicit path injection cleanly
func executeStrategy(client *NSEClient, symbol string, endpoint FilingsEndpoint, workerCount int, globalDataDir string) ([]byte, error) {
    var outputDir string

    // If Rust provides an explicit global data directory path, anchor it instantly!
    // Otherwise, use the bulletproof path resolution fallback logic for normal CLI runs.
    if globalDataDir != "" {
        outputDir = filepath.Join(globalDataDir, symbol, "nse_"+endpoint.Name())
        if err := os.MkdirAll(outputDir, 0755); err != nil {
            return nil, fmt.Errorf("failed creating explicit global target directory: %w", err)
        }
    } else {
        // Mount automated directory path right away: data/{symbol}/{api_name}
        baseDir, err := buildSaveDirectory(symbol, endpoint.Name())
        if err != nil {
            return nil, fmt.Errorf("failed creating directories: %w", err)
        }

        // Convert whatever path baseDir generated into a true absolute path clean-cut representation
        absPath, err := filepath.Abs(baseDir)
        if err != nil {
            return nil, fmt.Errorf("failed to compute absolute path context: %w", err)
        }

        // If "downloader/data" is anywhere in the computed absolute filesystem string, 
        // we split the path right at the root module block and anchor it back to "stock-app/data" safely.
        if strings.Contains(absPath, filepath.Join("downloader", "data")) {
            // Splitting at the exact platform-native representation of "downloader" segment
            parts := strings.Split(absPath, filepath.Join("downloader", "data"))
            // parts[0] is now guaranteed to be the clean absolute path straight to: /Users/aseem/Project/stock-app/
            outputDir = filepath.Join(parts[0], "data", symbol, "nse_"+endpoint.Name())
            
            // Regenerate the directory structures safely at the true root coordinates
            if err := os.MkdirAll(outputDir, 0755); err != nil {
                return nil, fmt.Errorf("failed generating unified parent directory mapping: %w", err)
            }
        } else {
            outputDir = absPath
        }
    }

    // 🛡️ INTERCEPT CHART STRATEGY: Handle Multi-Timeframe logic dynamically
    if endpoint.Name() == "historical-chart-data" {
        chartAPI, ok := endpoint.(HistoricalChartAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for HistoricalChartAPI")
        }

        directives := chartAPI.ParseMultiTimeframes(symbol)
        for _, dir := range directives {
            targetURL := dir.DownloadURL[12:]
            fmt.Printf("[scrape] 📈 Fetching historical market trend timeline: %s\n", dir.Period)

            req, err := http.NewRequest("GET", targetURL, nil)
            if err != nil {
                return nil, err
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
        return nil, nil // 🎯 Return empty bytes for special sub-file query loops
    }

    // 🛡️ INTERCEPT PEER COMPARISON STRATEGY: Matrix Combination Generator Loop
    if endpoint.Name() == "peer-comparison-matrix" {
        peerAPI, ok := endpoint.(PeerComparisonAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for PeerComparisonAPI")
        }

        combos := peerAPI.GetCombinations(symbol)
        fmt.Printf("[scrape] 📊 Running grid sweeper across %d distinct valuation peer matrix variants...\n", len(combos))

        for _, item := range combos {
            req, err := http.NewRequest("GET", item.URL, nil)
            if err != nil {
                return nil, err
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
                resp.Body.Close()
                continue
            }

            peerBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                continue
            }

            matrixPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", item.FileName))
            if err := os.WriteFile(matrixPath, peerBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "[scrape] ❌ Failed writing peer matrix %s: %v\n", item.FileName, err)
            }

            time.Sleep(150 * time.Millisecond)
        }
        return nil, nil // 🎯 Return empty bytes for special sub-file query loops
    }

    // ============================================================================
    // STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL OTHER ENDPOINTS
    // ============================================================================
    apiURL := endpoint.BuildURL(symbol)
    req, err := http.NewRequest("GET", apiURL, nil)
    if err != nil {
        return nil, err
    }
    req.Header.Set("User-Agent", UserAgent)
    req.Header.Set("Referer", Referer)
    req.Header.Set("Accept", "*/*")

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
    fmt.Printf("[scrape] 📝 Archiving raw response array payload to: %s\n", metaJSONPath)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        fmt.Fprintf(os.Stderr, "[scrape] ⚠️ Warning: Failed saving metadata JSON file: %v\n", err)
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    fmt.Printf("[scrape] Strategy '%s' identified %d files for %s.\n", endpoint.Name(), len(records), symbol)

    if len(records) == 0 {
        return rawBytes, nil // 🎯 JSON only endpoint has 0 downstream worker files! Return early.
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
    return rawBytes, nil // 🎯 Return the verified rawBytes alongside normal completion tracking loops!
}