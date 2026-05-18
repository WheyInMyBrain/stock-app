package scrape_nse

import "io"

// UniversalRecord standardizes the data layout so the downloader doesn't care which API it came from.
type UniversalRecord struct {
	Period      string // Can be Year (2024-2025) or Quarter End Date (31-Dec-2025)
	DownloadURL string
}

// FilingsEndpoint is the blueprint for any NSE API strategy you add in the future.
type FilingsEndpoint interface {
	Name() string                                            // E.g., "annual-reports" or "corporate-financial-results"
	BuildURL(symbol string) string                           // Constructs the target API URL
	ParseResponse(body io.Reader) ([]UniversalRecord, error) // Translates its specific JSON structure
}