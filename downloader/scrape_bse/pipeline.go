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

const (
    ColorBlue  = "\033[96m"
    ColorRed   = "\033[31m"
    ColorReset = "\033[0m"
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

// ExecuteWithWarmClient serves as the blazing-fast daemon gateway for the BSE stack.
func ExecuteWithWarmClient(client *BSEClient, symbol string, workerCount int, targetApi string, globalDataDir string, onlyJson bool) (string, error) {
    // 🔵 Unified Log: Smart Search Tracker Initiation
    fmt.Printf("%s{BSE}  🔍 Performing smart search lookup for ticker token: %s...%s\n", ColorBlue, symbol, ColorReset)
    scripCode, err := GetScripCode(client, symbol, globalDataDir)
    if err != nil {
        return "", fmt.Errorf("BSE identifier mapping failed: %w", err)
    }
    // 🔵 Unified Log: Resolution Handshake Successful
    fmt.Printf("%s{BSE} 🎯 Successfully mapped %s ----> BSE Scrip Code: %s%s\n", ColorBlue, symbol, scripCode, ColorReset)

    // 2. Dynamic endpoint array loaded from endpoints.go
    endpoints := GetAllEndpoints()
    var capturedJSON string

    for _, endpoint := range endpoints {
        // If a specific API is requested, bypass everything that doesn't match its endpoint name!
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        // 🔵 Unified Log: Sequential Pipeline Run Target Notification
        fmt.Printf("\n%s{BSE} 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, endpoint.Name(), ColorReset)

        // Execute each strategy using the shared, authenticated client and resolved scripCode
        rawBytes, err := executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir, onlyJson)
        if err != nil {
            // 🚨 Fault Isolation: Marked cleanly in Red
            fmt.Fprintf(os.Stderr, "%s{BSE} ⚠️ Error running pipeline %s: %v%s\n", ColorRed, endpoint.Name(), err, ColorReset)
            return "", err
        }

        // If the execution pulled valid network data bytes, cast them to a string reference frame
        if len(rawBytes) > 0 {
            capturedJSON = string(rawBytes)
        }
    }

    return capturedJSON, nil
}

// ExecuteAll serves as the single execution gateway from main.go for the BSE legacy pipeline network.
func ExecuteAll(symbol string, workerCount int, targetApi string, globalDataDir string, onlyJson bool) error {
    client, err := NewBSEClient()
    if err != nil {
        return fmt.Errorf("BSE session initialization failed: %w", err)
    }

    // 🔵 Unified Log: CLI Smart Search Lookup Tracker
    fmt.Printf("%s{BSE} 🔍 Performing smart search lookup for ticker token: %s...%s\n", ColorBlue, symbol, ColorReset)
    scripCode, err := GetScripCode(client, symbol, globalDataDir)
    if err != nil {
        return fmt.Errorf("BSE identifier mapping failed: %w", err)
    }
    // 🔵 Unified Log: CLI Resolution Handshake Successful
    fmt.Printf("%s{BSE} 🎯 Successfully mapped %s ----> BSE Scrip Code: %s%s\n", ColorBlue, symbol, scripCode, ColorReset)

    endpoints := GetAllEndpoints()
    if len(endpoints) == 0 {
        fmt.Printf("%s{BSE} 📌 No active BSE endpoint strategies registered yet.%s\n", ColorBlue, ColorReset)
        return nil
    }

    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }

        // 🔵 Unified Log: CLI Mode Pipeline Execution Notice
        fmt.Printf("\n%s{BSE} 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, endpoint.Name(), ColorReset)

        // 🎯 FIXED: Forward onlyJson into the execution strategy to short-circuit corporate document downloads
        _, _ = executeStrategy(client, symbol, scripCode, endpoint, workerCount, globalDataDir, onlyJson)
    }

    return nil
}

// executeStrategy maps out the processing loop safely.
// Updated signature syntax schema to return ([]byte, error) natively up the tracking context chain.
func executeStrategy(client *BSEClient, symbol, scripCode string, endpoint BSEFilingsEndpoint, workerCount int, globalDataDir string, onlyJson bool) ([]byte, error) {
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
            // 🔵 Unified Log: Blue institutional deal tracking
            fmt.Printf("%s{BSE} 📊 Fetching institutional market transaction layer: %s%s\n", ColorBlue, dir.Period, ColorReset)

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
                // 🚨 Fault Isolation: Red dropout notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Connection error dropped deal fetch for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            if resp.StatusCode != http.StatusOK {
                // 🚨 Fault Isolation: Red API rejection notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ BSE API rejected deal entry %s, status code: %d%s\n", ColorRed, dir.Period, resp.StatusCode, ColorReset)
                resp.Body.Close()
                continue
            }

            dealBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                // 🚨 Fault Isolation: Red stream fault notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Read failed for transaction stream row %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            dealPath := filepath.Join(outputDir, fmt.Sprintf("%s.json", dir.Period))
            if err := os.WriteFile(dealPath, dealBytes, 0644); err != nil {
                // 🚨 Fault Isolation: Red cache write notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Failed writing transaction file to disk %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
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
            // 🔵 Unified Log: Blue market metrics processing tracker
            fmt.Printf("%s{BSE} 📈 Processing and transforming tracking metrics: %s%s\n", ColorBlue, dir.Period, ColorReset)

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
                // 🚨 Fault Isolation: Red connection failure notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Connection failure for chart horizon %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            chartBytes, err := io.ReadAll(resp.Body)
            resp.Body.Close()
            if err != nil {
                // 🚨 Fault Isolation: Red buffer stream fault notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Read failed for chart stream %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            if err := chartAPI.ProcessAndNormalize(outputDir, dir.Period, chartBytes); err != nil {
                // 🚨 Fault Isolation: Red alignment matrix notice
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Transformation loop failed for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
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
    // 🔵 Unified Log: Blue payload storage notification
    fmt.Printf("%s{BSE} 📝 Archiving raw response array payload to: %s%s\n", ColorBlue, metaJSONPath, ColorReset)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        // 🚨 Fault Isolation: Red write warnings for missing filesystems
        fmt.Fprintf(os.Stderr, "%s{BSE} ⚠️ Warning: Failed saving metadata JSON file: %v%s\n", ColorRed, err, ColorReset)
    }

    // ============================================================================
    // 🎯 THE MASTER ONLY-JSON SHORT-CIRCUIT BREAKPOINT
    // ============================================================================
    if onlyJson {
        // 🔵 Unified Log: Blue layout pass short-circuit indicator
        fmt.Printf("%s{BSE} 🟢 Only-JSON mode active for '%s'. Safely bypassing worker document scraping queues.%s\n", ColorBlue, endpoint.Name(), ColorReset)
        return rawBytes, nil 
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    // 🔵 Unified Log: Blue records synchronization overview
    fmt.Printf("%s{BSE} Strategy '%s' identified %d files for %s.%s\n", ColorBlue, endpoint.Name(), len(records), symbol, ColorReset)

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
            // 🔵 Unified Log: Blue row exclusion notice
            fmt.Printf("%s{BSE} ⚠️ Skipping entry '%s': Invalid or empty download URL string.%s\n", ColorBlue, row.Period, ColorReset)
            continue
        }

        if row.DownloadURL[:4] != "http" {
            // 🔵 Unified Log: Blue protocol security warning
            fmt.Printf("%s{BSE} ⚠️ Skipping entry '%s': Unsupported url prefix: %s%s\n", ColorBlue, row.Period, row.DownloadURL, ColorReset)
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