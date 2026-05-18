package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// BSESmartSearchRow matches the exact JSON signature you intercepted!
type BSESmartSearchRow struct {
	ID        string `json:"ID"`        // e.g., "IMFA"
	Text      string `json:"text"`      // e.g., "Indian Metals & Ferro Alloys Ltd 533047    IMFA"
	ScripCode int    `json:"scripcode"` // e.g., 533047 (Pristine native integer!)
	ISIN      string `json:"ISIN"`      // e.g., "INE919H01018"
}

func GetBSEScripCode(symbol string) (string, error) {
	targetSymbol := strings.TrimSpace(strings.ToUpper(symbol))

	// The pristine smart search API endpoint you discovered
	apiURL := fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ListScripSmartSearch_ng/w?searchString=%s", targetSymbol)

	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return "", err
	}

	// 🛡️ Standard BSE domain firewall bypass parameters
	req.Header.Set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
	req.Header.Set("Accept", "application/json, text/plain, */*")
	req.Header.Set("Origin", "https://www.bseindia.com")
	req.Header.Set("Referer", "https://www.bseindia.com/") 

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("BSE smart search rejected request with status code: %d", resp.StatusCode)
	}

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	// Parse the naked JSON array response natively
	var rows []BSESmartSearchRow
	if err := json.Unmarshal(bodyBytes, &rows); err != nil {
		return "", fmt.Errorf("json parse error: %w. Raw body: %s", err, string(bodyBytes))
	}

	// Look for an exact match on the ID field matching our ticker symbol string token
	for _, row := range rows {
		if strings.TrimSpace(strings.ToUpper(row.ID)) == targetSymbol {
			// Convert the native integer to a clean string format for our URLs
			return fmt.Sprintf("%d", row.ScripCode), nil
		}
	}

	// Fallback safety: if no exact match is found, pull the first row's scripcode
	if len(rows) > 0 {
		return fmt.Sprintf("%d", rows[0].ScripCode), nil
	}

	return "", fmt.Errorf("could not resolve BSE Scrip Code for symbol: %s", symbol)
}

func main() {
	ticker := "IMFA"
	scripCode, err := GetBSEScripCode(ticker)
	if err != nil {
		fmt.Printf("❌ Lookup Failed: %v\n", err)
		return
	}

	fmt.Printf("✅ Lookup Success! Symbol: %s ----> BSE Scrip Code: %s\n", ticker, scripCode)
}