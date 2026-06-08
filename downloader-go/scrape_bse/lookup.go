package scrape_bse

import (
    "encoding/json"
    "fmt"
    "io"
    "net/http"
    "os"
    "path/filepath"
    "strings"
)

// BSESmartSearchRow tracks the exact structural layout signature returned by the Angular microservice.
type BSESmartSearchRow struct {
    ID        string `json:"ID"`        // e.g., TICKER NAME eg "IMFA"
    Text      string `json:"text"`      // e.g., COMPANY FULL NAME eg "Indian Metals & Ferro Alloys Ltd 533047    IMFA"
    ScripCode int    `json:"scripcode"` // e.g., SCRIPT CODE eg. 533047 (Native structural integer!)
    ISIN      string `json:"ISIN"`      // e.g., ??? eg. "INE919H01018"
}

// GetScripCode executes a smart-suggest index sweep to resolve text tokens to numerical BSE markers.
// 🎯 CACHE-OPTIMIZED: Added globalDataDir to intercept local cached snapshots before checking online.
func GetScripCode(client *BSEClient, symbol string, globalDataDir string) (string, error) {
    targetSymbol := strings.TrimSpace(strings.ToUpper(symbol))

    // 🎯 DETERMINE UNIFIED STORAGE DIRECTORY PATH
    var baseDir string
    if globalDataDir != "" {
        baseDir = filepath.Join(globalDataDir, symbol, "bse_chart-symbol-metadata")
    } else {
        baseDir = filepath.Join("data", symbol, "bse_chart-symbol-metadata")
    }
    
    cacheFilePath := filepath.Join(baseDir, "endpoint-metadata.json")

    var bodyBytes []byte
	var err error

    // 🔍 STEP 1: CACHE-FIRST CHECK (Has this symbol lookup run before?)
    if _, statErr := os.Stat(cacheFilePath); statErr == nil {
        fmt.Printf("[bse_scrape] 💾 Local snapshot detected for %s metadata. Loading from disk instantly!\n", symbol)
        bodyBytes, err = os.ReadFile(cacheFilePath)
        if err != nil {
            return "", fmt.Errorf("failed to read local BSE metadata cache file: %w", err)
        }
    } else {
        // 📡 STEP 2: NETWORK FALLBACK (If cache is completely empty, pull it live)
        fmt.Printf("[bse_scrape] 📡 Local cache missing. Fetching live search token metadata online for %s...\n", symbol)
        
        apiURL := fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ListScripSmartSearch_ng/w?searchString=%s", targetSymbol)

        req, err := http.NewRequest("GET", apiURL, nil)
        if err != nil {
            return "", err
        }

        // Inject cross-domain headers safely
        req.Header.Set("User-Agent", UserAgent)
        req.Header.Set("Accept", "application/json, text/plain, */*")
        req.Header.Set("Origin", Origin)
        req.Header.Set("Referer", Referer)

        resp, err := client.HTTPClient.Do(req)
        if err != nil {
            return "", fmt.Errorf("bse scrip lookup network request dropped: %w", err)
        }
        defer resp.Body.Close()

        if resp.StatusCode != http.StatusOK {
            return "", fmt.Errorf("bse tracking search interface returned invalid status code: %d", resp.StatusCode)
        }

        bodyBytes, err = io.ReadAll(resp.Body)
        if err != nil {
            return "", err
        }

        // 🎯 STEP 3: PERSIST FILE DATA TO CACHE IMMEDIATELY
        if err := os.MkdirAll(baseDir, 0755); err == nil {
            if writeErr := os.WriteFile(cacheFilePath, bodyBytes, 0644); writeErr != nil {
                fmt.Fprintf(os.Stderr, "[bse_scrape] ⚠️ Warning: Failed writing metadata cache file: %v\n", writeErr)
            }
        }
    }

    // 🎯 STEP 4: PARSE STRUCT MATRIX RECORD ROWS
    var rows []BSESmartSearchRow
    if err := json.Unmarshal(bodyBytes, &rows); err != nil {
        return "", fmt.Errorf("failed decoding smart search serialization block: %w", err)
    }

    // Run validation check against target identifiers
    for _, row := range rows {
        if strings.TrimSpace(strings.ToUpper(row.ID)) == targetSymbol {
            return fmt.Sprintf("%d", row.ScripCode), nil
        }
    }

    // Secondary fallback mechanism: grab the initial lookup result row if an exact token match slips
    if len(rows) > 0 {
        return fmt.Sprintf("%d", rows[0].ScripCode), nil
    }

    return "", fmt.Errorf("unable to map or translate symbol %s into a valid BSE scrip code identifier", symbol)
}