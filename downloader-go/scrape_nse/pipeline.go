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
    ColorBlue  = "\033[96m"
    ColorRed   = "\033[31m"
    ColorReset = "\033[0m"
)

// ExecuteWithWarmClient coordinates the NSE API batch queue concurrently with absolute fault isolation
func ExecuteWithWarmClient(client *NSEClient, symbol string, workerCount int, targetApi string, globalDataDir string, fromTime string, onlyJson bool, telemetry interface{ WriteLine(string) }) (string, error) {
    endpoints := GetAllEndpoints()
    var capturedJSON string

    scripCode, _ := GetScripCode(client, symbol, globalDataDir)

    totalSteps := 0
    for _, endpoint := range endpoints {
        if targetApi == "" || endpoint.Name() == targetApi {
            totalSteps++
        }
    }

    sem := make(chan struct{}, 1)
    var wg sync.WaitGroup
    var mu sync.Mutex 

    currentStep := 0
    for _, endpoint := range endpoints {
        if targetApi != "" && endpoint.Name() != targetApi {
            continue
        }
        currentStep++ 

        wg.Add(1)
        go func(ep FilingsEndpoint, step int) {
            defer wg.Done()

            sem <- struct{}{}
            defer func() { <-sem }() 

            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|STATUS:START|STEP:%d/%d", ep.Name(), step, totalSteps))
            }

            rawBytes, err := executeStrategy(client, symbol, scripCode, ep, workerCount, globalDataDir, fromTime, onlyJson, telemetry, step, totalSteps)
            
            mu.Lock()
            defer mu.Unlock()

            if err != nil {
                // 🛡️ ISOLATED FAULT BOUNDARY: Log the drop but do not fail the overarching batch function
                fmt.Fprintf(os.Stderr, "%s{NSE} ⚠️ Error running warm pipeline %s: %v%s\n", ColorRed, ep.Name(), err, ColorReset)
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

// ExecuteAll manages standalone CLI runs for NSE strategies concurrently with pacing controls
func ExecuteAll(symbol string, workerCount int, targetApi string, globalDataDir string, fromTime string, onlyJson bool) error {
    client, err := NewNSEClient()
    if err != nil {
        return fmt.Errorf("NSE session initialization failed: %w", err)
    }

    // 🔵 Unified Log: Token Lookup Tracking
    fmt.Printf("%s{NSE} 🔍 Resolving dynamic ticker token mapping for: %s...%s\n", ColorBlue, symbol, ColorReset)
    scripCode, err := GetScripCode(client, symbol, globalDataDir)
    if err != nil {
        return fmt.Errorf("NSE identifier mapping failed: %w", err)
    }
    // 🔵 Unified Log: Resolution Handshake Completed
    fmt.Printf("%s{NSE} 🎯 Successfully mapped %s ----> NSE Token ID: %s%s\n", ColorBlue, symbol, scripCode, ColorReset)

    endpoints := GetAllEndpoints()
    if len(endpoints) == 0 {
        fmt.Printf("%s{NSE} ℹ️ No active NSE endpoint strategies registered yet.%s\n", ColorBlue, ColorReset)
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
    sem := make(chan struct{}, 2)
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
        go func(ep FilingsEndpoint, step int) {
            defer wg.Done()

            // 🛑 ACQUIRE SLOT: Blocks if 2 strategies are already running across the parallel stack
            sem <- struct{}{}
            defer func() { <-sem }() // 🟢 RELEASE SLOT

            // 🔵 Unified Log: Sequential Pipeline Execution Start
            fmt.Printf("\n%s{NSE} 🌀 Running downloader for target endpoint: %s%s\n", ColorBlue, ep.Name(), ColorReset)

            // 🎯 SIGNATURE COMPLIANCE: Pass nil for telemetry, along with step context trackers to satisfy compiler requirements
            _, err := executeStrategy(client, symbol, scripCode, ep, workerCount, globalDataDir, fromTime, onlyJson, nil, step, totalSteps)
            
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
            // 📡 Safely push unified progress to Rust: GO_TELEMETRY|API:x|FILE:y|PCT:z|STEP:a/b
            ptr.telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:%s|PCT:%.1f|STEP:%d/%d", ptr.apiName, ptr.filename, pct, ptr.currentStep, ptr.totalSteps))
        }
    }
    return n, err
}

func executeStrategy(
    client *NSEClient, 
    symbol string, 
    scripCode string, 
    endpoint FilingsEndpoint, 
    workerCount int, 
    globalDataDir string, 
    fromTime string, 
    onlyJson bool,
    telemetry interface{ WriteLine(string) },
    currentStep int,
    totalSteps int,
) ([]byte, error) {
    var outputDir string

    if globalDataDir != "" {
        outputDir = filepath.Join(globalDataDir, symbol, "nse_"+endpoint.Name())
        if err := os.MkdirAll(outputDir, 0755); err != nil {
            return nil, fmt.Errorf("failed creating explicit global target directory: %w", err)
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
            outputDir = filepath.Join(parts[0], "data", symbol, "nse_"+endpoint.Name())
            if err := os.MkdirAll(outputDir, 0755); err != nil {
                return nil, fmt.Errorf("failed generating unified parent directory mapping: %w", err)
            }
        } else {
            outputDir = absPath
        }
    }

    // ============================================================================
    // 🛡️ INTERCEPT CHART STRATEGY: Handle Multi-Timeframe logic sequentially
    // ============================================================================
    if endpoint.Name() == "historical-chart-data" {
        chartAPI, ok := endpoint.(HistoricalChartAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for HistoricalChartAPI")
        }

        directives := chartAPI.ParseMultiTimeframes(symbol)
        for _, dir := range directives {
            targetURL := dir.DownloadURL[12:]
            filename := fmt.Sprintf("%s.json", dir.Period)
            fmt.Printf("%s{NSE} 📈 Fetching historical market trend timeline: %s%s\n", ColorBlue, dir.Period, ColorReset)

            req, err := http.NewRequest("GET", targetURL, nil)
            if err != nil {
                return nil, err
            }
            req.Header.Set("User-Agent", UserAgent)
            req.Header.Set("Referer", Referer)
            req.Header.Set("Accept", "*/*")

            resp, err := client.HTTPClient.Do(req)
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s{NSE} ❌ Chart fetch dropped for %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
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
                fmt.Fprintf(os.Stderr, "%s{NSE} ❌ Read failed for chart %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
                continue
            }

            // 🎯 TELEMETRY FLUSH: Explicit completion ticket allocation (FIXED: localName -> filename)
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), filename, currentStep, totalSteps))
            }

            tfPath := filepath.Join(outputDir, filename)
            if err := os.WriteFile(tfPath, chartBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "%s{NSE} ❌ Failed saving chart file %s: %v%s\n", ColorRed, dir.Period, err, ColorReset)
            }

            time.Sleep(100 * time.Millisecond)
        }
        return nil, nil 
    }

    // ============================================================================
    // 🛡️ INTERCEPT PEER COMPARISON STRATEGY: Matrix Combination Generator sequentially
    // ============================================================================
    if endpoint.Name() == "peer-comparison-matrix" {
        peerAPI, ok := endpoint.(PeerComparisonAPI)
        if !ok {
            return nil, fmt.Errorf("failed type assertion for PeerComparisonAPI")
        }

        combos := peerAPI.GetCombinations(symbol)
        fmt.Printf("%s{NSE} 📊 Running grid sweeper across %d distinct valuation peer matrix variants...%s\n", ColorBlue, len(combos), ColorReset)

        for _, item := range combos {
            filename := fmt.Sprintf("%s.json", item.FileName)
            req, err := http.NewRequest("GET", item.URL, nil)
            if err != nil {
                return nil, err
            }
            req.Header.Set("User-Agent", UserAgent)
            req.Header.Set("Referer", Referer)
            req.Header.Set("Accept", "*/*")

            resp, err := client.HTTPClient.Do(req)
            if err != nil {
                fmt.Fprintf(os.Stderr, "%s{NSE} ❌ Peer matrix drop for %s: %v%s\n", ColorRed, item.FileName, err, ColorReset)
                continue
            }

            if resp.StatusCode != http.StatusOK {
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

            peerBytes, err := io.ReadAll(tracker)
            resp.Body.Close()
            if err != nil {
                continue
            }

            // 🎯 TELEMETRY FLUSH: Explicit completion ticket allocation (FIXED: localName -> filename)
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), filename, currentStep, totalSteps))
            }

            matrixPath := filepath.Join(outputDir, filename)
            if err := os.WriteFile(matrixPath, peerBytes, 0644); err != nil {
                fmt.Fprintf(os.Stderr, "%s{NSE} ❌ Failed writing peer matrix %s: %v%s\n", ColorRed, item.FileName, err, ColorReset)
            }

            time.Sleep(100 * time.Millisecond)
        }
        return nil, nil 
    }

    // ============================================================================
    // STANDARD 1-TO-1 FILE DOWNLOAD PIPELINE FOR ALL OTHER ENDPOINTS (SEQUENTIAL)
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
        telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:endpoint-metadata.json|PCT:100.0|STEP:%d/%d", endpoint.Name(), currentStep, totalSteps))
    }

    metaJSONPath := filepath.Join(outputDir, "endpoint-metadata.json")
    fmt.Printf("%s{NSE} 📝 Archiving raw response array payload to: %s%s\n", ColorBlue, metaJSONPath, ColorReset)
    if err := os.WriteFile(metaJSONPath, rawBytes, 0644); err != nil {
        fmt.Fprintf(os.Stderr, "%s{NSE} ⚠️ Warning: Failed saving metadata JSON file: %v%s\n", ColorRed, err, ColorReset)
    }

    if onlyJson {
        fmt.Printf("%s{NSE} 🟢 Only-JSON mode active for '%s'. Safely bypassing worker document scraping queues.%s\n", ColorBlue, endpoint.Name(), ColorReset)
        return rawBytes, nil 
    }

    bodyReader := bytes.NewReader(rawBytes)
    records, err := endpoint.ParseResponse(bodyReader)
    if err != nil {
        return nil, fmt.Errorf("failed parsing data payload for %s: %w", endpoint.Name(), err)
    }

    fmt.Printf("%s{NSE} Strategy '%s' identified %d files for %s.%s\n", ColorBlue, endpoint.Name(), len(records), symbol, ColorReset)

    if len(records) == 0 {
        return rawBytes, nil 
    }

    for _, row := range records {
        if row.DownloadURL == "" || row.DownloadURL == "-" || len(row.DownloadURL) < 8 {
            continue
        }

        if row.DownloadURL[:4] != "http" {
            continue
        }

        ext := filepath.Ext(row.DownloadURL)
        if ext == "" {
            ext = ".xml"
        }

        localName := fmt.Sprintf("%s%s", row.Period, ext)
        fullDiskPath := filepath.Join(outputDir, localName)

        if _, err := os.Stat(fullDiskPath); err == nil {
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), localName, currentStep, totalSteps))
            }
            continue
        }

        fileReq, err := http.NewRequest("GET", row.DownloadURL, nil)
        if err != nil {
            continue
        }
        fileReq.Header.Set("User-Agent", UserAgent)
        fileReq.Header.Set("Referer", Referer)

        fileResp, err := client.HTTPClient.Do(fileReq)
        if err != nil {
            continue
        }

        if fileResp.StatusCode != http.StatusOK {
            fileResp.Body.Close()
            continue
        }

        out, err := os.Create(fullDiskPath)
        if err != nil {
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

        if err == nil {
            if telemetry != nil {
                telemetry.WriteLine(fmt.Sprintf("GO_TELEMETRY|EXCH:NSE|API:%s|FILE:%s|PCT:100.0|STEP:%d/%d", endpoint.Name(), localName, currentStep, totalSteps))
            }
        }

        time.Sleep(100 * time.Millisecond)
    }

    return rawBytes, nil 
}