package scrape_bse

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
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
func GetScripCode(client *BSEClient, symbol string) (string, error) {
	targetSymbol := strings.TrimSpace(strings.ToUpper(symbol))

	// Dynamic lookup query string formatting targeting the api subdomain
	apiURL := fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ListScripSmartSearch_ng/w?searchString=%s", targetSymbol)

	req, err := http.NewRequest("GET", apiURL, nil)
	if err != nil {
		return "", err
	}

	// Inject the specific cross-domain validation headers handled by our package constants
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

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

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