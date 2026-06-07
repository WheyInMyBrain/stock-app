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

// ExecuteWithWarmClient coordinates the BSE API batch queue concurrently with absolute fault isolation
func ExecuteWithWarmClient(client *BSEClient, symbol string, workerCount int, targetApi string, globalDataDir string, onlyJson bool, telemetry interface{ WriteLine(string) }) (string, error) {
    fmt.Printf("%s{BSE}  🔍 Performing smart search lookup for ticker token: %s...%s\n", ColorBlue, symbol, ColorReset)
    scripCode, err := GetScripCode(client, symbol, globalDataDir)
    if err != nil {
        return "", fmt.Errorf("BSE identifier mapping failed: %w", err)
    }
    fmt.Printf("%s{BSE} 🎯 Successfully mapped %s ----> BSE Scrip Code: %s%s\n", ColorBlue, symbol, scripCode, ColorReset)

    endpoints := GetAllEndpoints()
    totalSteps := 0
    for _, endpoint := range endpoints {
        if targetApi == "" || endpoint.Name() == targetApi {
            totalSteps++
        }
    }

    sem := make(chan struct{}, 2)
    var wg sync.WaitGroup
    var mu sync.Mutex 
    var capturedJSON string

    currentStep := 0
    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }
        currentStep++ 

        wg.Add(1)
        go func(ep BSEFilingsEndpoint, step int) {
            defer wg.Done()

            sem <- struct{}{}
            defer func() { <-sem }() 

            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|STATUS:START|STEP:%d/%d", ep.Name(), step, totalSteps))
            }

            fmt.Printf("\n%s{BSE} 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, ep.Name(), ColorReset)

            rawBytes, err := executeStrategy(client, symbol, scripCode, ep, workerCount, globalDataDir, onlyJson, telemetry, step, totalSteps)
            
            mu.Lock()
            defer mu.Unlock()

            if err != nil {
                // 🛡️ ISOLATED FAULT BOUNDARY: Log the drop but do not fail the overarching batch function
                fmt.Fprintf(os.Stderr, "%s{BSE} ⚠️ Error running pipeline %s: %v%s\n", ColorRed, ep.Name(), err, ColorReset)
                return
            }

            if len(rawBytes) > 0 {
                capturedJSON = string(rawBytes)
            }
        }(endpoint, currentStep)
    }

    wg.Wait()
    return capturedJSON, nil
}

// ExecuteAll manages standalone CLI runs for BSE strategies concurrently with pacing controls
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

    // 1. Pre-calculate total steps to maintain consistent step math contexts
    totalSteps := 0
    for _, endpoint := range endpoints {
        if targetApi == "" || endpoint.Name() == targetApi {
            totalSteps++
        }
    }

    // 🎯 FIREWALL SEMAPHORE: Restricts simultaneous active endpoints to 2 to safeguard network limits
    sem := make(chan struct{}, 1)
    var wg sync.WaitGroup
    var mu sync.Mutex
    var firstErr error

    currentStep := 0
    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }
        currentStep++

        wg.Add(1)
        // 🚀 MULTIPROCESSING: Spawn each independent strategy in parallel
        go func(ep BSEFilingsEndpoint, step int) {
            defer wg.Done()

            // 🛑 ACQUIRE SLOT: Blocks if 2 strategies are already running across the parallel stack
            sem <- struct{}{}
            defer func() { <-sem }() // 🟢 RELEASE SLOT

            // 🔵 Unified Log: CLI Mode Pipeline Execution Notice
            fmt.Printf("\n%s{BSE} 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, ep.Name(), ColorReset)

            // 🎯 SIGNATURE COMPLIANCE: Pass nil for telemetry, along with step context trackers to satisfy compiler requirements
            _, err := executeStrategy(client, symbol, scripCode, ep, workerCount, globalDataDir, onlyJson, nil, step, totalSteps)
            
            mu.Lock()
            if err != nil && firstErr == nil {
                firstErr = err // Safely store first broken context trace without concurrent overwrite races
            }
            mu.Unlock()
        }(endpoint, currentStep)
    }

    wg.Wait() // Wait for all concurrent routines to conclude operations
    return firstErr
}

type progressTrackingReader struct {
    io.Reader
    apiName     string
    filename    string
    totalBytes  int64
    readBytes   int64
    currentStep int
    totalSteps  int
    telemetry   interface{ WriteLine(string) }
}

func (ptr *progressTrackingReader) Read(p []byte) (int, error) {
    n, err := ptr.Reader.Read(p)
    if n > 0 {
        ptr.readBytes += int64(n)
        if ptr.totalBytes > 0 && ptr.telemetry != nil {
            pct := (float64(ptr.readBytes) / float64(ptr.totalBytes)) * 100.0
            ptr.telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:%s|PCT:%.1f|STEP:%d/%d", ptr.apiName, ptr.filename, pct, ptr.currentStep, ptr.totalSteps))
        }
    }
    return n, err
}

func executeStrategy(
    client *BSEClient,
    symbol, scripCode string,
    endpoint BSEFilingsEndpoint,
    workerCount int,
    globalDataDir string, 
    onlyJson bool,
    telemetry interface{ WriteLine(string) },
    currentStep int,
    totalSteps int,
) ([]byte, error) {
    var outputDir string

    if globalDataDir != "" {
        outputDir = filepath.Join(globalDataDir, symbol, "bse_"+endpoint.Name())
        if err := os.MkdirAll(outputDir, 0755); err != nil {
            return nil, fmt.Errorf("failed creating explicit global BSE target directory: %w", err)
        }
    } else {
        baseDir, err := buildSaveDirectory(symbol, endpoint.Name())
        if err != nil {
            return nil, fmt.Errorf("failed creating directories: %w", err)
        }

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
    // 🛡️ INTERCEPT DEALS STRATEGY: Handle Bulk and Block dynamics sequentially
    // ============================================================================
    if endpoint.Name() == "bulk-block-deals" {
        dealsAPI, ok := endpoint.(BSEBulkBlockDealsAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for BSEBulkBlockDealsAPI")
        }

        directives := dealsAPI.ParseDeals(scripCode)
        var lastDealsBytes []byte

        for _, dir := range directives {
            filename := fmt.Sprintf("%s.json", dir.Period)
            targetURL := dir.DownloadURL[15:]
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
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Connection error dropped deal fetch for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            if resp.StatusCode != http.StatusOK {
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ BSE API rejected deal entry %s, status code: %d%s\n", ColorRed, dir.Period, resp.StatusCode, ColorReset)
                resp.Body.Close()
                continue
            }

            tracker := &progressTrackingReader{
                Reader:      resp.Body,
                apiName:     endpoint.Name(),
                filename:    filename,
                totalBytes:  resp.ContentLength,
                currentStep: currentStep,
                totalSteps:  totalSteps,
                telemetry:   telemetry,
            }

            dealBytes, err := io.ReadAll(tracker)
            resp.Body.Close()
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Read failed for transaction stream row %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            // 🎯 TELEMETRY FLUSH: Explicit completion ticket allocation
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), filename, currentStep, totalSteps))
            }

            dealPath := filepath.Join(outputDir, filename)
            if err := os.WriteFile(dealPath, dealBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Failed writing transaction file to disk %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
            }

            lastDealsBytes = dealBytes
            time.Sleep(100 * time.Millisecond)
        }
        return lastDealsBytes, nil 
    }

    // ============================================================================
    // 🛡️ INTERCEPT CHART STRATEGY: Handle History chart multi-horizons sequentially
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
            filename := fmt.Sprintf("%s.json", dir.Period)
            targetURL := dir.DownloadURL[17:] 
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
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Connection failure for chart horizon %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            tracker := &progressTrackingReader{
                Reader:      resp.Body,
                apiName:     endpoint.Name(),
                filename:    filename,
                totalBytes:  resp.ContentLength,
                currentStep: currentStep,
                totalSteps:  totalSteps,
                telemetry:   telemetry,
            }

            chartBytes, err := io.ReadAll(tracker)
            resp.Body.Close()
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Read failed for chart stream %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            // 🎯 TELEMETRY FLUSH: Explicit completion ticket allocation
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), filename, currentStep, totalSteps))
            }

            if err := chartAPI.ProcessAndNormalize(outputDir, dir.Period, chartBytes); err != nil {
                fmt.Fprintf(os.Stderr, "%s{BSE} ❌ Transformation loop failed for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
            }

            lastChartBytes = chartBytes
            time.Sleep(100 * time.Millisecond)
        }
        return lastChartBytes, nil 
    }

    // ============================================================================
    // STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL BSE ENDPOINTS (SEQUENTIAL)
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

    tracker := &progressTrackingReader{
        Reader:      resp.Body,
        apiName:     endpoint.Name(),
        filename:    "endpoint-metadata.json",
        totalBytes:  resp.ContentLength,
        currentStep: currentStep,
        totalSteps:  totalSteps,
        telemetry:   telemetry,
    }

    rawBytes, err := io.ReadAll(tracker)
    if err != nil {
        return nil, fmt.Errorf("failed reading body bytes: %w", err)
    }

    // 🎯 TELEMETRY FLUSH: Explicit completion marker for index JSON payloads
    if telemetry != nil {
        telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:endpoint-metadata.json|PCT:100.0|STEP:%d/%d", endpoint.Name(), currentStep, totalSteps))
    }

    metaJSONPath := filepath.Join(outputDir, "endpoint-metadata.json")
    fmt.Printf("%s{BSE} 📝 Archiving raw response array payload to: %s%s\n", ColorBlue, metaJSONPath, ColorReset)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        fmt.Fprintf(os.Stderr, "%s{BSE} ⚠️ Warning: Failed saving metadata JSON file: %v%s\n", ColorRed, err, ColorReset)
    }

    if onlyJson {
        fmt.Printf("%s{BSE} 🟢 Only-JSON mode active for '%s'. Safely bypassing worker document scraping queues.%s\n", ColorBlue, endpoint.Name(), ColorReset)
        return rawBytes, nil 
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    fmt.Printf("%s{BSE} Strategy '%s' identified %d files for %s.%s\n", ColorBlue, endpoint.Name(), len(records), symbol, ColorReset)

    if len(records) == 0 {
        return rawBytes, nil 
    }

    for _, row := range records {
        if row.DownloadURL == "" || row.DownloadURL == "-" || len(row.DownloadURL) < 8 {
            fmt.Printf("%s{BSE} ⚠️ Skipping entry '%s': Invalid or empty download URL string.%s\n", ColorBlue, row.Period, ColorReset)
            continue
        }

        if row.DownloadURL[:4] != "http" {
            fmt.Printf("%s{BSE} ⚠️ Skipping entry '%s': Unsupported url prefix: %s%s\n", ColorBlue, row.Period, row.DownloadURL, ColorReset)
            continue
        }

        ext := filepath.Ext(row.DownloadURL)
        if ext == "" {
            ext = ".xml" 
        }

        localName := fmt.Sprintf("%s%s", row.Period, ext)
        fullDiskPath := filepath.Join(outputDir, localName)

        if _, err := os.Stat(fullDiskPath); err == nil {
            fmt.Printf("{BSE} ⏭️ Skipped (Already Downloaded): %s\n", localName)
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), localName, currentStep, totalSteps))
            }
            continue
        }

        fmt.Printf("{BSE} ⏳ Downloading: %s\n", localName)
        fileReq, err := http.NewRequest("GET", row.DownloadURL, nil)
        if err != nil {
            fmt.Printf("{BSE} ❌ Request fail for %s: %v\n", localName, err)
            continue
        }
        fileReq.Header.Set("User-Agent", UserAgent)
        fileReq.Header.Set("Origin", Origin)
        fileReq.Header.Set("Referer", Referer)

        fileResp, err := client.HTTPClient.Do(fileReq)
        if err != nil {
            fmt.Printf("{BSE} ❌ Connection error for %s: %v\n", localName, err)
            continue
        }

        if fileResp.StatusCode != http.StatusOK {
            fmt.Printf("{BSE} ❌ Server rejected %s: HTTP Status Code %d\n", localName, fileResp.StatusCode)
            fileResp.Body.Close()
            continue
        }

        out, err := os.Create(fullDiskPath)
        if err != nil {
            fmt.Printf("{BSE} ❌ Disk create error for %s: %v\n", localName, err)
            fileResp.Body.Close()
            continue
        }

        fileTracker := &progressTrackingReader{
            Reader:      fileResp.Body,
            apiName:     endpoint.Name(),
            filename:    localName,
            totalBytes:  fileResp.ContentLength,
            currentStep: currentStep,
            totalSteps:  totalSteps,
            telemetry:   telemetry,
        }

        _, err = io.Copy(out, fileTracker)
        out.Close()
        fileResp.Body.Close()

        if err != nil {
            fmt.Printf("{BSE} ❌ Write processing fail for %s: %v\n", localName, err)
        } else {
            fmt.Printf("{BSE} ✅ Finished: %s\n", localName)
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:BSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), localName, currentStep, totalSteps))
            }
        }

        time.Sleep(100 * time.Millisecond)
    }

    return rawBytes, nil
}