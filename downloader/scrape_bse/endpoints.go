package scrape_bse

import (
	"encoding/json"
	"fmt"
	"io"
	"strings"
	"time"
)

// GetAllEndpoints registers all active Bombay Stock Exchange scraping strategies.
func GetAllEndpoints() []BSEFilingsEndpoint {
	return []BSEFilingsEndpoint{
		BSECorporateHeaderAPI{},
		BSEScripHeaderDataAPI{},
		BSEStockTradingDataAPI{},
		BSEHistoricalChartAPI{},
		BSEBulkBlockDealsAPI{},
		BSEFinancialResultsAPI{},
		BSEBoardMeetingsAPI{},
		BSEShareholderMeetingsAPI{},
		BSEVotingResultsAPI{},
		BSECorporateActionsAPI{},
		BSEShareholdingPatternAPI{},
		BSECorporateGovernanceAPI{},
		BSEInvestorComplaintsAPI{},
		BSEPeerGroupAPI{},
		BSECorporateInfoAPI{},
	}
}

// ============================================================================
// STRATEGY 1: Corporate Details Master Header Matrix
// ============================================================================
type BSECorporateHeaderAPI struct{}

func (c BSECorporateHeaderAPI) Name() string { return "corporate-details-header" }

func (c BSECorporateHeaderAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ComHeadernew/w?quotetype=&scripcode=%s&seriesid=", scripCode)
}

func (c BSECorporateHeaderAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically archives this payload as "endpoint-metadata.json".
	// We return nil because there are no files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 2: Core Pricing and Scrip Meta Status Data
// ============================================================================
type BSEScripHeaderDataAPI struct{}

func (s BSEScripHeaderDataAPI) Name() string { return "scrip-pricing-header" }

func (s BSEScripHeaderDataAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/getScripHeaderData/w?Debtflag=&scripcode=%s&seriesid=", scripCode)
}

func (s BSEScripHeaderDataAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically archives this payload as "endpoint-metadata.json".
	// We return nil because there are no files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 3: Real-Time Market Depth & Trading Turnover
// ============================================================================
type BSEStockTradingDataAPI struct{}

func (t BSEStockTradingDataAPI) Name() string { return "live-trading-turnover" }

func (t BSEStockTradingDataAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/StockTrading/w?flag=&quotetype=EQ&scripcode=%s", scripCode)
}

func (t BSEStockTradingDataAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
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

// ============================================================================
// STRATEGY 6: Financial Results & XBRL Data Document Tracker
// ============================================================================
type BSEFinancialResultsAPI struct{}

func (f BSEFinancialResultsAPI) Name() string { return "financial-results-docs" }

func (f BSEFinancialResultsAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/Result_Arch_ng/w?scrip_cd=%s", scripCode)
}

// BSEResultsPayload maps the structural JSON array wrapper schema returned by BSE
type BSEResultsPayload struct {
	Table []struct {
		Quarter         string      `json:"Quarter"`
		StandXBRL       interface{} `json:"stand_xbrl_link"` // Can be string or null
		ConsoXBRL       interface{} `json:"conso_xbrl_link"` // Can be string or null
		WebURL          string      `json:"Weburl"`
		FilingDateTime  string      `json:"Filing_Date_Time"`
	} `json:"Table"`
}

func (f BSEFinancialResultsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload BSEResultsPayload
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var records []UniversalRecord

	for _, row := range payload.Table {
		// Clean and sanitize the quarter string token to create pristine local filenames
		cleanQuarter := strings.ReplaceAll(row.Quarter, ";", "_")
		cleanQuarter = strings.ReplaceAll(cleanQuarter, ":", "-")
		cleanQuarter = strings.ReplaceAll(cleanQuarter, "/", "-")

		// 1. Process the standard Web Reporting URL
		if row.WebURL != "" {
			records = append(records, UniversalRecord{
				Period:      fmt.Sprintf("%s_WebReport", cleanQuarter),
				DownloadURL: row.WebURL,
			})
		}

		// Helper function to extract relative paths from dynamic JSON interfaces safely
		parseRelativeLink := func(linkInterface interface{}) string {
			if linkInterface == nil {
				return ""
			}
			strVal, ok := linkInterface.(string)
			if !ok {
				return ""
			}
			return strings.TrimSpace(strVal)
		}

		// 2. Process Standalone XBRL Link
		if standPath := parseRelativeLink(row.StandXBRL); standPath != "" {
			records = append(records, UniversalRecord{
				Period:      fmt.Sprintf("%s_Standalone_XBRL", cleanQuarter),
				DownloadURL: "https://www.bseindia.com" + standPath, // Prefix host to relative paths
			})
		}

		// 3. Process Consolidated XBRL Link
		if consoPath := parseRelativeLink(row.ConsoXBRL); consoPath != "" {
			records = append(records, UniversalRecord{
				Period:      fmt.Sprintf("%s_Consolidated_XBRL", cleanQuarter),
				DownloadURL: "https://www.bseindia.com" + consoPath, // Prefix host to relative paths
			})
		}
	}

	return records, nil
}

// ============================================================================
// STRATEGY 7: Corporate Board Meetings Schedule Tracker
// ============================================================================
type BSEBoardMeetingsAPI struct{}

func (b BSEBoardMeetingsAPI) Name() string { return "board-meetings" }
func (b BSEBoardMeetingsAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/BoardMeeting/w?scripcode=%s", scripCode)
}
func (b BSEBoardMeetingsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Pipeline archives the raw payload as metadata automatically
}

// ============================================================================
// STRATEGY 8: Shareholder Meetings (AGM / EGM) Calendar
// ============================================================================
type BSEShareholderMeetingsAPI struct{}

func (s BSEShareholderMeetingsAPI) Name() string { return "shareholder-meetings" }
func (s BSEShareholderMeetingsAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/ShareHolderMeeting/w?scripcode=%s", scripCode)
}
func (s BSEShareholderMeetingsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Pipeline archives the raw payload as metadata automatically
}

// ============================================================================
// STRATEGY 9: Dynamic Voting Results Document Vault Downloader (2-Year Window)
// ============================================================================
type BSEVotingResultsAPI struct{}

func (v BSEVotingResultsAPI) Name() string { return "voting-results-docs" }
func (v BSEVotingResultsAPI) BuildURL(scripCode string) string {
	// 1. Calculate time horizons dynamically
	now := time.Now()
	twoYearsAgo := now.AddDate(-2, 0, 0)

	// 2. Format precisely to required bse API format layout style: DD/MM/YYYY
	toDateStr := now.Format("02/01/2006")
	fromDateStr := twoYearsAgo.Format("02/01/2006")

	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/VotingResults/w?fromdt=%s&todt=%s&type=0&scripcode=%s", fromDateStr, toDateStr, scripCode)
}

// BSEVotingPayload maps out the structural voting resolution array response layout
type BSEVotingPayload struct {
	Table []struct {
		MasterID      int         `json:"Fld_MasterID"`
		Description   string      `json:"Description"`
		MeetingDate   string      `json:"Fld_MeetingDate"`
		SrNo          int         `json:"fld_srno"`
		XMLNameLink   interface{} `json:"Fld_XMLName"`       // Can be string or null
		PDFDocument   interface{} `json:"PDFDocumentname"`   // Can be string or null
		FilingTime    string      `json:"Filing_Date_Time"`
	} `json:"Table"`
}

func (v BSEVotingResultsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload BSEVotingPayload
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var records []UniversalRecord

	// Helper to extract string text from variant JSON interfaces safely
	parseLink := func(val interface{}) string {
		if val == nil {
			return ""
		}
		str, ok := val.(string)
		if !ok {
			return ""
		}
		return strings.TrimSpace(str)
	}

	for _, row := range payload.Table {
		// Clean and sanitize descriptive strings to create valid file signatures
		cleanDesc := strings.ReplaceAll(row.Description, " ", "_")
		
		// Create a consistent base naming format string
		baseName := fmt.Sprintf("ID_%d_%s_SrNo_%d", row.MasterID, cleanDesc, row.SrNo)

		// 1. Extract and schedule the HTML/XML structural voting data ledger file download
		if xmlPath := parseLink(row.XMLNameLink); xmlPath != "" {
			records = append(records, UniversalRecord{
				Period:      fmt.Sprintf("%s_DataLedger", baseName),
				DownloadURL: "https://www.bseindia.com" + xmlPath, // Prepend the official base domain
			})
		}

		// 2. Extract and schedule the official executive PDF Scrip summary report file download
		if pdfPath := parseLink(row.PDFDocument); pdfPath != "" {
			records = append(records, UniversalRecord{
				Period:      fmt.Sprintf("%s_ReportDoc", baseName),
				DownloadURL: "https://www.bseindia.com" + pdfPath, // Prepend the official base domain
			})
		}
	}

	return records, nil
}

// ============================================================================
// STRATEGY 10: Corporate Actions (Dividends, Splits, Bonuses)
// ============================================================================
type BSECorporateActionsAPI struct{}

func (c BSECorporateActionsAPI) Name() string { return "corporate-actions" }
func (c BSECorporateActionsAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/CorporateAction/w?scripcode=%s", scripCode)
}
func (c BSECorporateActionsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Automatically archives the raw object array to endpoint-metadata.json
}

// ============================================================================
// STRATEGY 11: Shareholding Pattern (SHP) Document Downloader
// ============================================================================
type BSEShareholdingPatternAPI struct{}

func (s BSEShareholdingPatternAPI) Name() string { return "shareholding-pattern-docs" }
func (s BSEShareholdingPatternAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/SHPQNewFormat/w?scripcode=%s", scripCode)
}

type BSESHPPayload struct {
	Table []struct {
		Year     string      `json:"yr"`
		Quarter  string      `json:"qtr"`
		Status   string      `json:"status"`
		XBRLUrl  interface{} `json:"xbrlurl"` // HTML rendering view of the data map
	} `json:"Table"`
}

func (s BSEShareholdingPatternAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload BSESHPPayload
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var records []UniversalRecord
	for _, row := range payload.Table {
		if row.XBRLUrl == nil {
			continue
		}
		pathStr, ok := row.XBRLUrl.(string)
		if !ok || strings.TrimSpace(pathStr) == "" {
			continue
		}

		// Sanitize quarter information strings to build safe local names
		cleanQtr := strings.ReplaceAll(row.Quarter, " ", "_")
		cleanYr := strings.ReplaceAll(row.Year, " ", "")
		cleanYr = strings.ReplaceAll(cleanYr, "-", "_")

		localToken := fmt.Sprintf("SHP_%s_%s_%s", cleanQtr, cleanYr, row.Status)

		records = append(records, UniversalRecord{
			Period:      localToken,
			DownloadURL: "https://www.bseindia.com" + strings.TrimSpace(pathStr),
		})
	}
	return records, nil
}

// ============================================================================
// STRATEGY 12: Corporate Governance (CG) Archive Document Downloader
// ============================================================================
type BSECorporateGovernanceAPI struct{}

func (g BSECorporateGovernanceAPI) Name() string { return "corporate-governance-docs" }
func (g BSECorporateGovernanceAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/CGArchivewise/w?scripcode=%s", scripCode)
}

type BSECGPayload struct {
	Table []struct {
		QuarterID interface{} `json:"Fld_QuarterId"`
		Year      string      `json:"Year"`
		Quarter   string      `json:"qtr"`
		Status    string      `json:"status"`
		XBRLUrl   interface{} `json:"xbrlurl"` // Path link pointing directly to compliance .xml files
	} `json:"Table"`
}

func (g BSECorporateGovernanceAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload BSECGPayload
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var records []UniversalRecord
	for _, row := range payload.Table {
		if row.XBRLUrl == nil {
			continue
		}
		pathStr, ok := row.XBRLUrl.(string)
		if !ok || strings.TrimSpace(pathStr) == "" {
			continue
		}

		// Handle variations where QuarterId can arrive as a float or string float seamlessly
		qtrIDStr := ""
		switch v := row.QuarterID.(type) {
		case float64:
			qtrIDStr = fmt.Sprintf("%.2f", v)
		case string:
			qtrIDStr = v
		}

		cleanQtr := strings.ReplaceAll(row.Quarter, " ", "_")
		cleanYr := strings.ReplaceAll(row.Year, " ", "")
		cleanYr = strings.ReplaceAll(cleanYr, "-", "_")

		localToken := fmt.Sprintf("CG_%s_%s_ID_%s_%s", cleanQtr, cleanYr, qtrIDStr, row.Status)

		records = append(records, UniversalRecord{
			Period:      localToken,
			DownloadURL: "https://www.bseindia.com" + strings.TrimSpace(pathStr),
		})
	}
	return records, nil
}

// ============================================================================
// STRATEGY 13: Investor Complaints Document Downloader
// ============================================================================
type BSEInvestorComplaintsAPI struct{}

func (i BSEInvestorComplaintsAPI) Name() string { return "investor-complaints-docs" }
func (i BSEInvestorComplaintsAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/XbrlInvestorComplaint/w?scripcode=%s", scripCode)
}

type BSEComplaintsPayload struct {
	Table []struct {
		Year      string      `json:"yr"`
		Quarter   string      `json:"qtr"`
		QuarterID interface{} `json:"qtrid"`
		Status    string      `json:"status"`
		XBRLUrl   interface{} `json:"xbrlurl"` // Relative link pointing directly to the compliance file asset
	} `json:"Table"`
}

func (i BSEInvestorComplaintsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload BSEComplaintsPayload
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var records []UniversalRecord
	for _, row := range payload.Table {
		if row.XBRLUrl == nil {
			continue
		}
		pathStr, ok := row.XBRLUrl.(string)
		if !ok || strings.TrimSpace(pathStr) == "" {
			continue
		}

		// Normalize QuarterID variants gracefully
		qtrIDStr := ""
		switch v := row.QuarterID.(type) {
		case float64:
			qtrIDStr = fmt.Sprintf("%.2f", v)
		case string:
			qtrIDStr = v
		}

		cleanQtr := strings.ReplaceAll(row.Quarter, " ", "_")
		cleanYr := strings.ReplaceAll(row.Year, " ", "")
		cleanYr = strings.ReplaceAll(cleanYr, "-", "_")

		localToken := fmt.Sprintf("Complaints_%s_%s_ID_%s_%s", cleanQtr, cleanYr, qtrIDStr, row.Status)

		records = append(records, UniversalRecord{
			Period:      localToken,
			DownloadURL: "https://www.bseindia.com" + strings.TrimSpace(pathStr),
		})
	}
	return records, nil
}

// ============================================================================
// STRATEGY 14: Industry Peer Group Valuation Matrix
// ============================================================================
type BSEPeerGroupAPI struct{}

func (p BSEPeerGroupAPI) Name() string { return "peer-valuation-matrix" }
func (p BSEPeerGroupAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/PeerGpCom/w?scripcode=%s&scripcomare=", scripCode)
}
func (p BSEPeerGroupAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Pipeline saves the complete response table as endpoint-metadata.json
}

// ============================================================================
// STRATEGY 15: Corporate Information Master Directory
// ============================================================================
type BSECorporateInfoAPI struct{}

func (c BSECorporateInfoAPI) Name() string { return "corporate-info-directory" }
func (c BSECorporateInfoAPI) BuildURL(scripCode string) string {
	return fmt.Sprintf("https://api.bseindia.com/BseIndiaAPI/api/CorpInfoNew/w?scripcode=%s", scripCode)
}
func (c BSECorporateInfoAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Pipeline saves the complete response table as endpoint-metadata.json
}