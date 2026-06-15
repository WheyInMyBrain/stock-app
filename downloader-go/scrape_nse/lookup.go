package scrape_nse

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
)

// NSESymbolDynamicRow tracks the layout signature returned by the charting endpoint
type NSESymbolDynamicRow struct {
	Symbol    string `json:"symbol"`    // e.g., "IMFA-EQ"
	ScripCode string `json:"scripcode"` // e.g., "19235" (Stored natively as a string here!)
}

type NSESymbolDynamicResponse struct {
	Status bool                  `json:"status"`
	Data   []NSESymbolDynamicRow `json:"data"`
}

// GetScripCode handles caching the lookup data locally to prevent redundant web calls
func GetScripCode(client *NSEClient, symbol string, globalDataDir string) (string, error) {
	// Directly append the required -EQ suffix onto the raw symbol token string
	targetSymbol := symbol + "-EQ"

	// 🎯 DETERMINE STORAGE DIRECTORY PATH
	var baseDir string
	if globalDataDir != "" {
		baseDir = filepath.Join(globalDataDir, symbol, "nse_chart-symbol-metadata")
	} else {
		baseDir = filepath.Join("data", symbol, "nse_chart-symbol-metadata")
	}
	
	cacheFilePath := filepath.Join(baseDir, "endpoint-metadata.json")

	var bodyBytes []byte
	var err error

	// 🔍 STEP 1: CACHE-FIRST CHECK (Is it downloaded already?)
	if _, statErr := os.Stat(cacheFilePath); statErr == nil {
		fmt.Printf("[nse_scrape] 💾 Local snapshot detected for %s metadata. Loading from disk instantly!\n", symbol)
		bodyBytes, err = os.ReadFile(cacheFilePath)
		if err != nil {
			return "", fmt.Errorf("failed to read local NSE metadata cache file: %w", err)
		}
	} else {
		// 🌐 STEP 2: NETWORK FALLBACK (If file doesn't exist, download it)
		fmt.Printf("[nse_scrape] 📡 Local cache missing. Fetching live token metadata online for %s...\n", symbol)
		
		apiURL := fmt.Sprintf("https://charting.nseindia.com/v1/exchanges/symbolsDynamic?symbol=%s&segment=", targetSymbol)
		req, err := http.NewRequest("GET", apiURL, nil)
		if err != nil {
			return "", err
		}

		req.Header.Set("User-Agent", UserAgent)
		req.Header.Set("Accept", "application/json, text/plain, */*")
		req.Header.Set("Referer", Referer)

		resp, err := client.HTTPClient.Do(req)
		if err != nil {
			return "", fmt.Errorf("nse scrip lookup network request dropped: %w", err)
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			return "", fmt.Errorf("nse tracking search interface returned invalid status code: %d", resp.StatusCode)
		}

		bodyBytes, err = io.ReadAll(resp.Body)
		if err != nil {
			return "", err
		}

		// 🎯 ARCHIVE TO DISK LAYER: Create folders and save the raw JSON for next time!
		if err := os.MkdirAll(baseDir, 0755); err == nil {
			if writeErr := os.WriteFile(cacheFilePath, bodyBytes, 0644); writeErr != nil {
				fmt.Fprintf(os.Stderr, "[nse_scrape] ⚠️ Warning: Failed writing metadata cache file: %v\n", writeErr)
			}
		}
	}

	// 🎯 STEP 3: PARSE AND EXTRACT SCRIPCODE
	var response NSESymbolDynamicResponse
	if err := json.Unmarshal(bodyBytes, &response); err != nil {
		return "", fmt.Errorf("failed decoding symbol metadata serialization block: %w", err)
	}

	// Match token against our configured identifier format target string
	for _, row := range response.Data {
		if strings.TrimSpace(strings.ToUpper(row.Symbol)) == strings.ToUpper(targetSymbol) {
			return row.ScripCode, nil
		}
	}

	if len(response.Data) > 0 {
		return response.Data[0].ScripCode, nil
	}

	return "", fmt.Errorf("unable to map or translate symbol %s into a valid NSE scrip code identifier", symbol)
}