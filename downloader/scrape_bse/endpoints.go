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
		BSEHistoricalChartAPI{},
		BSEBulkBlockDealsAPI{},
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

// ============================================================================
// STRATEGY 4: Historical Chart Data Multi-Timeframe Sweeper
// ============================================================================
type BSEHistoricalChartAPI struct{}

func (h BSEHistoricalChartAPI) Name() string { return "historical-chart-data" }

func (h BSEHistoricalChartAPI) BuildURL(scripCode string) string {
	// Dummy fallback implementation to satisfy interface boundaries
	return ""
}

func (h BSEHistoricalChartAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Dummy fallback implementation to satisfy interface boundaries
	return nil, nil
}

// Custom ParseMultiHorizons generates direct query configurations for all targeted tracking horizons
func (h BSEHistoricalChartAPI) ParseMultiHorizons(scripCode string) []UniversalRecord {
	horizons := []string{"1D", "5D", "1M", "12M"}
	var results []UniversalRecord

	for _, flag := range horizons {
		url := fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/StockReachGraph/w?scripcode=%s&flag=%s&fromdate=&todate=&seriesid=", scripCode, flag)
		
		results = append(results, UniversalRecord{
			Period:      flag,
			DownloadURL: "BSE_CHART_FETCH:" + url,
		})
	}
	return results
}

// ============================================================================
// STRATEGY 5: Historical Bulk & Block Deal Sweeper
// ============================================================================
type BSEBulkBlockDealsAPI struct{}

func (b BSEBulkBlockDealsAPI) Name() string { return "bulk-block-deals" }

func (b BSEBulkBlockDealsAPI) BuildURL(scripCode string) string {
	// Dummy fallback implementation to satisfy interface boundaries
	return ""
}

func (b BSEBulkBlockDealsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Dummy fallback implementation to satisfy interface boundaries
	return nil, nil
}

// Custom ParseDeals packs both Bulk (type=1) and Block (type=2) directives dynamically
func (b BSEBulkBlockDealsAPI) ParseDeals(scripCode string) []UniversalRecord {
	return []UniversalRecord{
		{Period: "Bulk_Deals", DownloadURL: "BSE_DEAL_FETCH:https://api.bseindia.com/BseIndiaAPI/api/BulkblockDeal/w?fromdt=&todt=&type=1&scripcode=" + scripCode},
		{Period: "Block_Deals", DownloadURL: "BSE_DEAL_FETCH:https://api.bseindia.com/BseIndiaAPI/api/BulkblockDeal/w?fromdt=&todt=&type=2&scripcode=" + scripCode},
	}
}

