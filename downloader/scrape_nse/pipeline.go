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

const (
    ColorBlue  = "\033[34m"
    ColorRed   = "\033[31m"
    ColorReset = "\033[0m"
)

// ExecuteWithWarmClient now logs errors uniformly in Red with the downloader prefix
func ExecuteWithWarmClient(client *NSEClient, symbol string, workerCount int, targetApi string, globalDataDir string, fromTime string, onlyJson bool) (string, error) {
    endpoints := GetAllEndpoints()
    var capturedJSON string

    scripCode, _ := GetScripCode(client, symbol, globalDataDir)

    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        // 🎯 CAPTURE BOTH THE RAW BYTES AND ERROR FROM THE STRATEGY PASS
        rawBytes, err := executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir, fromTime, onlyJson)
        if err != nil {
            // 🚨 Fault Isolation: Marked cleanly in Red
            fmt.Fprintf(os.Stderr, "%s[GO-downloader] ⚠️ Error running warm pipeline %s: %v%s\n", ColorRed, endpoint.Name(), err, ColorReset)
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
// All informational messages are now strictly wrapped in Blue with the unified terminal tracker prefix
func ExecuteAll(symbol string, workerCount int, targetApi string, globalDataDir string, fromTime string, onlyJson bool) error {
    client, err := NewNSEClient()
    if err != nil {
        return fmt.Errorf("NSE session initialization failed: %w", err)
    }

    // 🔵 Unified Log: Token Lookup Tracking
    fmt.Printf("%s[GO-downloader] 🔍 Resolving dynamic ticker token mapping for: %s...%s\n", ColorBlue, symbol, ColorReset)
    scripCode, err := GetScripCode(client, symbol, globalDataDir)
    if err != nil {
        return fmt.Errorf("NSE identifier mapping failed: %w", err)
    }
    // 🔵 Unified Log: Resolution Handshake Completed
    fmt.Printf("%s[GO-downloader] 🎯 Successfully mapped %s ----> NSE Token ID: %s%s\n", ColorBlue, symbol, scripCode, ColorReset)

    endpoints := GetAllEndpoints()
    if len(endpoints) == 0 {
        fmt.Printf("%s[GO-downloader] ℹ️ No active NSE endpoint strategies registered yet.%s\n", ColorBlue, ColorReset)
        return nil
    }

    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        // 🔵 Unified Log: Sequential Pipeline Execution Start
        fmt.Printf("\n%s[GO-downloader] 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, endpoint.Name(), ColorReset)

        _, _ = executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir, fromTime, onlyJson)
    }

    return nil
}

// executeStrategy is unexported (private) to keep the package API surface tiny.
// Added globalDataDir to signature to handle explicit path injection cleanly
func executeStrategy(client *NSEClient, symbol string, scripCode string, endpoint FilingsEndpoint, workerCount int, globalDataDir string, fromTime string, onlyJson bool) ([]byte, error) {
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
            fmt.Printf("%s[GO-downloader] 📈 Fetching historical market trend timeline: %s%s\n", ColorBlue, dir.Period, ColorReset)

            req, err := http.NewRequest("GET", targetURL, nil)
            if err != nil {
                return nil, err
            }
            req.Header.Set("User-Agent", UserAgent)
            req.Header.Set("Referer", Referer)
            req.Header.Set("Accept", "*/*")

            resp, err := client.HTTPClient.Do(req)
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Chart fetch dropped for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            if resp.StatusCode != http.StatusOK {
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Chart API rejected timeframe %s, status: %d%s\n", ColorRed, dir.Period, resp.StatusCode, ColorReset)
                resp.Body.Close()
                continue
            }

            chartBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Read failed for chart %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            // 🎯 FIX: Added ':' to create a proper short-variable declaration statement
            tfPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", dir.Period))
            if err := os.WriteFile(tfPath, chartBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Failed saving chart file %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
            }

            time.Sleep(150 * time.Millisecond)
        }
        return nil, nil 
    }

    // ============================================================================
    // 🛡️ INTERCEPT PEER COMPARISON STRATEGY: Matrix Combination Generator Loop
    // ============================================================================
    if endpoint.Name() == "peer-comparison-matrix" {
        peerAPI, ok := endpoint.(PeerComparisonAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for PeerComparisonAPI")
        }

        combos := peerAPI.GetCombinations(symbol)
        // 🔵 Unified Log: Blue grid sweeping optimization progress indicator
        fmt.Printf("%s[GO-downloader] 📊 Running grid sweeper across %d distinct valuation peer matrix variants...%s\n", ColorBlue, len(combos), ColorReset)

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
                // 🚨 Fault Isolation: Red connection breakdown
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Peer matrix drop for %s: %v%s\n", ColorRed, item.FileName, err, ColorReset)
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
                // 🚨 Fault Isolation: Red write breakdown
                fmt.Fprintf(os.Stderr, "%s[GO-downloader] ❌ Failed writing peer matrix %s: %v%s\n", ColorRed, item.FileName, err, ColorReset)
            }

            time.Sleep(150 * time.Millisecond)
        }
        return nil, nil 
    }

    // ============================================================================
    // STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL OTHER ENDPOINTS
    // ============================================================================
    apiURL := endpoint.BuildURL(symbol)
    if endpoint.Name() == "real-time-chart-delta" {
        lastTimeStr := "0"
        if strings.TrimSpace(fromTime) != "" {
            lastTimeStr = fromTime
            if len(lastTimeStr) == 13 {
                lastTimeStr = lastTimeStr[:10]
            }
        }
        apiURL = strings.ReplaceAll(apiURL, "FROM_TS_PLACEHOLDER", lastTimeStr)
    }
    if strings.Contains(apiURL, "SCRIP_TOKEN_PLACEHOLDER") {
        apiURL = strings.ReplaceAll(apiURL, "SCRIP_TOKEN_PLACEHOLDER", scripCode)
    }
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
    // 🔵 Unified Log: Blue disk payload caching tracking log
    fmt.Printf("%s[GO-downloader] 📝 Archiving raw response array payload to: %s%s\n", ColorBlue, metaJSONPath, ColorReset)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        // 🚨 Fault Isolation: Red warning for missing or protected filesystems
        fmt.Fprintf(os.Stderr, "%s[GO-downloader] ⚠️ Warning: Failed saving metadata JSON file: %v%s\n", ColorRed, err, ColorReset)
    }

    // 🎯 THE MASTER ONLY-JSON SHORT-CIRCUIT BREAKPOINT
    if onlyJson {
        // 🔵 Unified Log: Blue microsecond fast-pass confirmation
        fmt.Printf("%s[GO-downloader] 🟢 Only-JSON mode active for '%s'. Safely bypassing worker document scraping queues.%s\n", ColorBlue, endpoint.Name(), ColorReset)
        return rawBytes, nil 
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    // 🔵 Unified Log: Blue record count verification
    fmt.Printf("%s[GO-downloader] Strategy '%s' identified %d files for %s.%s\n", ColorBlue, endpoint.Name(), len(records), symbol, ColorReset)

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
            // 🔵 Unified Log: Blue row skipping notification
            fmt.Printf("%s[GO-downloader] ⚠️ Skipping entry '%s': Invalid or empty download URL string.%s\n", ColorBlue, row.Period, ColorReset)
            continue
        }

        if row.DownloadURL[:4] != "http" {
            // 🔵 Unified Log: Blue domain mismatch warning
            fmt.Printf("%s[GO-downloader] ⚠️ Skipping entry '%s': Unsupported url prefix: %s%s\n", ColorBlue, row.Period, row.DownloadURL, ColorReset)
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