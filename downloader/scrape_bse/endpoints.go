package scrape_bse

import (
	"fmt"
	"io"
)

// GetAllEndpoints registers all active Bombay Stock Exchange scraping strategies.
func GetAllEndpoints() []BSEFilingsEndpoint {
	return []BSEFilingsEndpoint{
		CorporateHeaderAPI{},
		ScripHeaderDataAPI{},
		StockTradingDataAPI{},
	}
}

// ============================================================================
// STRATEGY 1: Corporate Details Master Header Matrix
// ============================================================================
type CorporateHeaderAPI struct{}

func (c CorporateHeaderAPI) Name() string { return "corporate-details-header" }

func (c CorporateHeaderAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ComHeadernew/w?quotetype=&scripcode=%s&seriesid=", scripCode)
}

func (c CorporateHeaderAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically archives this payload as "endpoint-metadata.json".
	// We return nil because there are no files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 2: Core Pricing and Scrip Meta Status Data
// ============================================================================
type ScripHeaderDataAPI struct{}

func (s ScripHeaderDataAPI) Name() string { return "scrip-pricing-header" }

func (s ScripHeaderDataAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/getScripHeaderData/w?Debtflag=&scripcode=%s&seriesid=", scripCode)
}

func (s ScripHeaderDataAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically archives this payload as "endpoint-metadata.json".
	// We return nil because there are no files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 3: Real-Time Market Depth & Trading Turnover
// ============================================================================
type StockTradingDataAPI struct{}

func (t StockTradingDataAPI) Name() string { return "live-trading-turnover" }

func (t StockTradingDataAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/StockTrading/w?flag=&quotetype=EQ&scripcode=%s", scripCode)
}

func (t StockTradingDataAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically archives this payload as "endpoint-metadata.json".
	// We return nil because there are no files to schedule for downloading!
	return nil, nil
}