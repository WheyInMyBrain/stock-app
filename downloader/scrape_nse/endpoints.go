package scrape_nse

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"path/filepath"
	"time"
)

// GetAllEndpoints registers all the individual active scrapers.
func GetAllEndpoints() []FilingsEndpoint {
	return []FilingsEndpoint{
		AnnualReportsAPI{},
		FinancialResultsAPI{},
		AnnualReportsXbrlAPI{},
		CorporateGovernanceAPI{},
		CorporateAnnouncementsAPI{},
		BusinessSustainabilityAPI{},
		CorporateBoardMeetingsAPI{},
		CorporateActionsAPI{},
		InsiderPlanAPI{},
		InvestorComplaintsAPI{},
		HistoricalChartAPI{},
		SymbolDataAPI{},
		PeerComparisonAPI{},
		BulkAndBlockDealsAPI{},
		CorporateShareholdingMasterAPI{},
	}
}

// ============================================================================
// STRATEGY 1: Annual Reports
// ============================================================================
type AnnualReportsAPI struct{}

type nseAnnualReportRow struct {
	FromYear string `json:"fromYr"`
	ToYear   string `json:"toYr"`
	FileName string `json:"fileName"`
}

func (a AnnualReportsAPI) Name() string { return "annual-reports" }

func (a AnnualReportsAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/annual-reports?index=equities&symbol=%s", symbol)
}

func (a AnnualReportsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload struct {
		Data []nseAnnualReportRow `json:"data"`
	}
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range payload.Data {
		if row.FileName == "" {
			continue
		}
		results = append(results, UniversalRecord{
			Period:      fmt.Sprintf("%s-%s", row.FromYear, row.ToYear),
			DownloadURL: row.FileName,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 2: Financial Results (Quarterly / Audited Accounts)
// ============================================================================
type FinancialResultsAPI struct{}

type nseFinancialRow struct {
	ToDate       string `json:"toDate"`       // e.g., "31-Dec-2024"
	XBRL         string `json:"xbrl"`         // e.g., "https://nsearchives.nseindia.com/..."
	Consolidated string `json:"consolidated"` // e.g., "Consolidated" or "Non-Consolidated"
}

func (f FinancialResultsAPI) Name() string { return "corporates-financial-results" }

func (f FinancialResultsAPI) BuildURL(symbol string) string {
	// Clean, stripped version of your new URL layout (defaulting to quarterly records)
	return fmt.Sprintf("https://www.nseindia.com/api/corporates-financial-results?index=equities&symbol=%s&period=Quarterly", symbol)
}

func (f FinancialResultsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Decodes directly into a slice array [] instead of an object mapping wrapper!
	var rows []nseFinancialRow
	if err := json.NewDecoder(body).Decode(&rows); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range rows {
		if row.XBRL == "" {
			continue
		}

		// Prevent duplicate dates from overwriting each other by appending consolidated status
		// This turns file names cleanly into: "31-Dec-2024_Consolidated.xml"
		fileNameSuffix := "Standalone"
		if row.Consolidated == "Consolidated" {
			fileNameSuffix = "Consolidated"
		}

		uniquePeriodName := fmt.Sprintf("%s_%s", row.ToDate, fileNameSuffix)

		results = append(results, UniversalRecord{
			Period:      uniquePeriodName,
			DownloadURL: row.XBRL,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 3: Annual Reports XBRL
// ============================================================================
type AnnualReportsXbrlAPI struct{}

type nseAnnualXbrlRow struct {
	FromYear       string `json:"fromYr"`
	ToYear         string `json:"toYr"`
	XbrlAttachment string `json:"fileName"`
}

func (a AnnualReportsXbrlAPI) Name() string { return "annual-reports-xbrl" }

func (a AnnualReportsXbrlAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/annual-reports-xbrl?index=equities&symbol=%s", symbol)
}

func (a AnnualReportsXbrlAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload struct {
		Data []nseAnnualXbrlRow `json:"data"`
	}
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range payload.Data {
		if row.XbrlAttachment == "" {
			continue
		}
		results = append(results, UniversalRecord{
			Period:      fmt.Sprintf("%s-%s", row.FromYear, row.ToYear),
			DownloadURL: row.XbrlAttachment,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 4: Corporate Governance
// ============================================================================
type CorporateGovernanceAPI struct{}

type nseGovernanceRow struct {
	Date string `json:"date"` 
	XBRL string `json:"xbrl"` 
}

func (c CorporateGovernanceAPI) Name() string { return "corporate-governance-master" }

func (c CorporateGovernanceAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-governance-master?index=equities&symbol=%s", symbol)
}

func (c CorporateGovernanceAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload struct {
		Data []nseGovernanceRow `json:"data"`
	}
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range payload.Data {
		if row.XBRL == "" {
			continue
		}
		results = append(results, UniversalRecord{
			Period:      row.Date,
			DownloadURL: row.XBRL,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 5: Corporate Announcements (PDFs)
// ============================================================================
type CorporateAnnouncementsAPI struct{}

type nseAnnouncementRow struct {
	SeqID        string `json:"seq_id"`        // Unique sequential key tracking id (e.g., "106622626")
	Attachment   string `json:"attchmntFile"`  // URL target download file link
	Announcement string `json:"an_dt"`         // Timestamp (e.g., "14-May-2026 16:49:09")
}

func (c CorporateAnnouncementsAPI) Name() string { return "corporate-announcements" }

func (c CorporateAnnouncementsAPI) BuildURL(symbol string) string {
	// Clean, stripped version of your 5th API configuration path
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-announcements?index=equities&symbol=%s&reqXbrl=false", symbol)
}

func (c CorporateAnnouncementsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Decodes straight into a slice array slice since there is no data object wrapper
	var rows []nseAnnouncementRow
	if err := json.NewDecoder(body).Decode(&rows); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range rows {
		if row.Attachment == "" {
			continue // Avoid blank records
		}

		// Parse the timestamp slightly to remove illegal characters like colons (:) from filenames
		// "14-May-2026 16:49:09" turns into a clean segment: "14-May-2026_16-49-09"
		cleanDate := ""
		for _, char := range row.Announcement {
			if char == ':' {
				cleanDate += "-"
			} else if char == ' ' {
				cleanDate += "_"
			} else {
				cleanDate += string(char)
			}
		}

		// Ensure absolute file differentiation using the sequential tracker id
		// This builds names perfectly: "14-May-2026_16-49-09_ID-106622626.pdf"
		uniqueFileName := fmt.Sprintf("%s_ID-%s", cleanDate, row.SeqID)

		results = append(results, UniversalRecord{
			Period:      uniqueFileName,
			DownloadURL: row.Attachment,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 6: Business Sustainability (BRSR - Both PDF & XBRL)
// ============================================================================
type BusinessSustainabilityAPI struct{}

type nseSustainabilityRow struct {
	FromYear   int    `json:"fyFrom"` // Note: NSE provides these as raw numbers/integers!
	ToYear     int    `json:"fyTo"`
	Attachment string `json:"attachmentFile"` // PDF download link
	XBRL       string `json:"xbrlFile"`       // XBRL download link
}

func (b BusinessSustainabilityAPI) Name() string { return "corporate-bussiness-sustainabilitiy" }

func (b BusinessSustainabilityAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-bussiness-sustainabilitiy?index=equities&symbol=%s", symbol)
}

func (b BusinessSustainabilityAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Notice that this endpoint uses the outer {"data": [...]} wrapper again!
	var payload struct {
		Data []nseSustainabilityRow `json:"data"`
	}
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range payload.Data {
		// Construct the base period name, e.g., "2024-2025"
		basePeriod := fmt.Sprintf("%d-%d", row.FromYear, row.ToYear)

		// 1. Process the PDF Report if it exists
		if row.Attachment != "" {
			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_Report", basePeriod),
				DownloadURL: row.Attachment, // The pipeline automatically handles the .pdf extension
			})
		}

		// 2. Process the XBRL Data file if it exists
		if row.XBRL != "" {
			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_XBRL", basePeriod),
				DownloadURL: row.XBRL, // The pipeline automatically handles the .xml extension
			})
		}
	}
	return results, nil
}

// ============================================================================
// STRATEGY 7: Corporate Board Meetings (Multi-Format PDF, XBRL & iXBRL)
// ============================================================================
type CorporateBoardMeetingsAPI struct{}

type nseBoardMeetingRow struct {
	MeetingDate string `json:"bm_date"`      // Date of meeting (e.g., "05-Feb-2026")
	Timestamp   string `json:"bm_timestamp"` // Specific upload timing (e.g., "29-Jan-2026 10:21:14")
	Attachment  string `json:"attachment"`   // Can be either an XML file link OR a PDF file link
	IXBRL       string `json:"ixbrl"`        // Inline XBRL HTML link (can be null or blank)
}

func (cb CorporateBoardMeetingsAPI) Name() string { return "corporate-board-meetings" }

func (cb CorporateBoardMeetingsAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-board-meetings?index=equities&symbol=%s", symbol)
}

func (cb CorporateBoardMeetingsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Root JSON Array layout with no outer "data" wrapper
	var rows []nseBoardMeetingRow
	if err := json.NewDecoder(body).Decode(&rows); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range rows {
		// Clean illegal file characters (like spaces and colons) from the timestamp string
		// "29-Jan-2026 10:21:14" becomes "29-Jan-2026_10-21-14"
		cleanTimestamp := ""
		for _, char := range row.Timestamp {
			if char == ':' {
				cleanTimestamp += "-"
			} else if char == ' ' {
				cleanTimestamp += "_"
			} else {
				cleanTimestamp += string(char)
			}
		}

		// Create a shared base naming token using both dates to keep items isolated
		baseName := fmt.Sprintf("Meeting_%s_Filed_%s", row.MeetingDate, cleanTimestamp)

		// 1. Process the standard "attachment" (could be .pdf or .xml)
		if row.Attachment != "" {
			// Determine whether it's a structural XML asset or standard Document
			typeLabel := "Document"
			ext := filepath.Ext(row.Attachment)
			if ext == ".xml" {
				typeLabel = "XBRL"
			}

			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_%s", baseName, typeLabel),
				DownloadURL: row.Attachment, // Pipeline auto-appends proper extension from the URL
			})
		}

		// 2. Process the "ixbrl" interactive HTML layout if it exists
		if row.IXBRL != "" {
			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_iXBRL", baseName),
				DownloadURL: row.IXBRL, // Pipeline auto-appends .html from the URL extension
			})
		}
	}
	return results, nil
}

// ============================================================================
// STRATEGY 8: Corporate Actions
// ============================================================================
type CorporateActionsAPI struct{}

func (ca CorporateActionsAPI) Name() string { return "corporates-corporateActions" }

func (ca CorporateActionsAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/corporates-corporateActions?index=equities&symbol=%s", symbol)
}

func (ca CorporateActionsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The pipeline automatically saved the raw JSON file as "endpoint-metadata.json".
	// We return nil here because there are no external files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 9: Insider Trading Plans (Multi-Format XBRL & iXBRL)
// ============================================================================
type InsiderPlanAPI struct{}

type nseInsiderPlanRow struct {
	AppID          string `json:"appid"`          // Unique transaction id tracking sequence
	SubmissionDate string `json:"submissionDate"` // Timing layout e.g. "19-Mar-2026 19:02:44"
	Attachment     string `json:"attachment"`     // Raw XBRL target file link (.xml)
	IXBRL          string `json:"ixbrl"`          // Inline interactive file link (.html)
}

func (i InsiderPlanAPI) Name() string { return "corporate-insider-plan" }

func (i InsiderPlanAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-insider-plan?index=equities&symbol=%s", symbol)
}

func (i InsiderPlanAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Root JSON Array layout with no outer "data" wrapping object
	var rows []nseInsiderPlanRow
	if err := json.NewDecoder(body).Decode(&rows); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range rows {
		// Strip out illegal file character paths (colons and spaces) from date string
		// "19-Mar-2026 19:02:44" becomes "19-Mar-2026_19-02-44"
		cleanDate := ""
		for _, char := range row.SubmissionDate {
			if char == ':' {
				cleanDate += "-"
			} else if char == ' ' {
				cleanDate += "_"
			} else {
				cleanDate += string(char)
			}
		}

		// Create a fully localized token signature base to protect file boundaries
		baseName := fmt.Sprintf("Plan_%s_App-%s", cleanDate, row.AppID)

		// 1. Queue up the raw structural data XBRL file asset if available
		if row.Attachment != "" {
			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_XBRL", baseName),
				DownloadURL: row.Attachment, // Core framework streams file and auto-appends .xml
			})
		}

		// 2. Queue up the parallel Interactive inline presentation asset if it exists
		if row.IXBRL != "" {
			results = append(results, UniversalRecord{
				Period:      fmt.Sprintf("%s_iXBRL", baseName),
				DownloadURL: row.IXBRL, // Core framework streams file and auto-appends .html
			})
		}
	}
	return results, nil
}

// ============================================================================
// STRATEGY 10: Investor Complaints (XBRLs)
// ============================================================================
type InvestorComplaintsAPI struct{}

type nseComplaintRow struct {
	Date string `json:"date"` // Ending quarter period e.g., "31-Dec-2024"
	XBRL string `json:"xbrl"` // Target download path string link
}

func (i InvestorComplaintsAPI) Name() string { return "investor-complaints" }

func (i InvestorComplaintsAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/investor-complaints?index=equities&symbol=%s", symbol)
}

func (i InvestorComplaintsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	var payload struct {
		Data []nseComplaintRow `json:"data"`
	}
	if err := json.NewDecoder(body).Decode(&payload); err != nil {
		return nil, err
	}

	var results []UniversalRecord
	for _, row := range payload.Data {
		if row.XBRL == "" {
			continue
		}

		// Pull the exact filename out of the web URL link string to extract its unique id block
		// E.g., "https://.../INVESTOR_1337629_04012025045624_WEB.xml" -> "INVESTOR_1337629_04012025045624_WEB.xml"
		webFileName := filepath.Base(row.XBRL)
		ext := filepath.Ext(webFileName)
		
		// Strip off the extension trailing string (.xml) to get just the clean string sequence
		rawToken := webFileName
		if len(ext) > 0 {
			rawToken = webFileName[:len(webFileName)-len(ext)]
		}

		// Incorporate the clear data metrics and the token to avoid system collisions
		// Result string signature turns into: "31-Dec-2024_File_INVESTOR_1337629_04012025045624_WEB"
		uniquePeriodLabel := fmt.Sprintf("%s_File_%s", row.Date, rawToken)

		results = append(results, UniversalRecord{
			Period:      uniquePeriodLabel,
			DownloadURL: row.XBRL,
		})
	}
	return results, nil
}

// ============================================================================
// STRATEGY 11: Historical Chart Coordinates Data (Multi-Timeframe)
// ============================================================================
type HistoricalChartAPI struct{}

func (h HistoricalChartAPI) Name() string { return "historical-chart-data" }

func (h HistoricalChartAPI) BuildURL(symbol string) string {
	// Dummy fallback implementation to satisfy the interface boundary
	return ""
}

func (h HistoricalChartAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// Dummy fallback implementation to satisfy the interface boundary
	return nil, nil
}

// Custom ParseMultiTimeframes builds the special fetch directives for all time horizons
func (h HistoricalChartAPI) ParseMultiTimeframes(symbol string) []UniversalRecord {
	timeframes := []string{"1D", "1W", "1M", "1Y", "5Y", "10Y", "30Y"}
	var results []UniversalRecord

	for _, tf := range timeframes {
		url := fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getSymbolChartData&symbol=%sEQN&days=%s", symbol, tf)
		
		// Pass the timeframe identifier in Period and use a CHART_FETCH: prefix trick
		results = append(results, UniversalRecord{
			Period:      tf,
			DownloadURL: "CHART_FETCH:" + url,
		})
	}
	return results
}

// ============================================================================
// STRATEGY 12: Fundamental Symbol Data Tracker
// ============================================================================
type SymbolDataAPI struct{}

func (s SymbolDataAPI) Name() string { return "symbol-core-data" }
func (s SymbolDataAPI) BuildURL(symbol string) string {
	return fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getSymbolData&marketType=N&series=EQ&symbol=%s", symbol)
}
func (s SymbolDataAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	return nil, nil // Pipeline automatically saves this via raw metadata file dump!
}

// ============================================================================
// STRATEGY 13: Cross-Sectional Peer Comparison Data Matrix (FULLY AUTOMATED)
// ============================================================================
type PeerComparisonAPI struct{}

func (p PeerComparisonAPI) Name() string { return "peer-comparison-matrix" }
func (p PeerComparisonAPI) BuildURL(symbol string) string { return "" }
func (p PeerComparisonAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) { return nil, nil }

// Struct to track dynamic multi-dimensional configuration inputs
type PeerDirective struct {
	FileName string
	URL      string
}

// Structural mapping model for the NSE Quarter API payload array format
type QuarterResponse struct {
	Label string `json:"label"`
	Value string `json:"value"` // This is what we need (e.g., "2025-12")
}

// Fetch real-time available data quarters directly from the NSE API tracking layers
func fetchDynamicQuarters(symbol string) []string {
	// 🎯 DYNAMIC TIMELINE QUERY: Grab the exact valid reporting quarters available right now
	url := fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonQuaters&symbol=%s", symbol)

	client := &http.Client{Timeout: 10 * time.Second}
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		fmt.Printf("⚠️  Failed to build quarter lookup blueprint request for %s: %v\n", symbol, err)
		return nil
	}

	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
	req.Header.Set("Accept", "application/json")

	resp, err := client.Do(req)
	if err != nil {
		fmt.Printf("⚠️  Network timeout pulling dynamic quarters timeline for %s: %v\n", symbol, err)
		return nil
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("⚠️  NSE Quarter API returned non-200 state context for %s: %d\n", symbol, resp.StatusCode)
		return nil
	}

	var parsedQuarters []QuarterResponse
	if err := json.NewDecoder(resp.Body).Decode(&parsedQuarters); err != nil {
		fmt.Printf("⚠️  Failed to decode dynamic quarter schema layout for %s: %v\n", symbol, err)
		return nil
	}

	// Isolate and extract just the values array
	var quarters []string
	for _, q := range parsedQuarters {
		if q.Value != "" {
			quarters = append(quarters, q.Value)
		}
	}

	return quarters
}

// Fetch real-time index alignments directly from the NSE API tracking layers
func fetchDynamicIndices(symbol string) []string {
	url := fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getIndexList&symbol=%s", symbol)

	client := &http.Client{Timeout: 10 * time.Second}
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		fmt.Printf("⚠️  Failed to create index lookup request for %s: %v\n", symbol, err)
		return nil
	}
	
	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
	req.Header.Set("Accept", "application/json")

	resp, err := client.Do(req)
	if err != nil {
		fmt.Printf("⚠️  Network timeout pulling dynamic indices for %s: %v\n", symbol, err)
		return nil
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil
	}

	var indices []string
	if err := json.NewDecoder(resp.Body).Decode(&indices); err != nil {
		return nil
	}

	return indices
}

func (p PeerComparisonAPI) GetCombinations(symbol string) []PeerDirective {
	// 🚀 STEP 1: Dynamically pull the latest available quarter nodes from the web
	quarters := fetchDynamicQuarters(symbol)
	
	// Fallback to maintain historical pipeline stability if the network fails
	if len(quarters) == 0 {
		fmt.Printf("ℹ️  Using default time-series fallback track matrix for stock: %s\n", symbol)
		quarters = []string{"2025-12", "2025-09", "2025-06", "2025-03"}
	}

	// 🚀 STEP 2: Dynamically pull the live index assignments
	indices := fetchDynamicIndices(symbol)
	
	if len(indices) == 0 {
		fmt.Printf("ℹ️  Using safe index fallback registry layout for stock tracking: %s\n", symbol)
		indices = []string{"NIFTY TOTAL MARKET"}
	}

	var directives []PeerDirective

	// Nest both dynamic collections together to map out our absolute collection parameters grid
	for _, q := range quarters {
		// 1. Core Industry Parameter Combo
		indURL := fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonData&symbol=%s&type=C&quarter=%s&param=industry&index=", symbol, q)
		directives = append(directives, PeerDirective{
			FileName: fmt.Sprintf("Industry_%s", q),
			URL:      indURL,
		})

		// 2. Specific Index Parameter Combos
		for _, idx := range indices {
			escapedIndex := strings.ReplaceAll(idx, " ", "%20")
			cleanIndexName := strings.ReplaceAll(idx, " ", "_")

			idxURL := fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonData&symbol=%s&type=C&quarter=%s&param=index&index=%s", symbol, q, escapedIndex)
			directives = append(directives, PeerDirective{
				FileName: fmt.Sprintf("Index_%s_%s", cleanIndexName, q),
				URL:      idxURL,
			})
		}
	}

	return directives
}

// ============================================================================
// STRATEGY 14: Historical Bulk & Block Deals (Last 2 Years)
// ============================================================================
type BulkAndBlockDealsAPI struct{}

func (b BulkAndBlockDealsAPI) Name() string { return "bulk-block-deals" }

func (b BulkAndBlockDealsAPI) BuildURL(symbol string) string {
	// 1. Grab current time vector bounds dynamically
	now := time.Now()
	twoYearsAgo := now.AddDate(-2, 0, 0) // Subtract exactly 2 years

	// 2. Format times precisely into the NSE required layout string: DD-MM-YYYY
	toDateStr := now.Format("02-01-2006")
	fromDateStr := twoYearsAgo.Format("02-01-2006")

	// 3. Synthesize the final query request path mapping
	return fmt.Sprintf("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getHistoricalBulkAndBlockData&symbol=%s&fromDate=%s&toDate=%s", symbol, fromDateStr, toDateStr)
}

func (b BulkAndBlockDealsAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// The central pipeline engine automatically records this payload as "endpoint-metadata.json".
	// We return nil here because there are no files to schedule for downloading!
	return nil, nil
}

// ============================================================================
// STRATEGY 15: Corporate Shareholding Pattern XML Master
// ============================================================================
type CorporateShareholdingMasterAPI struct{}

func (c CorporateShareholdingMasterAPI) Name() string { return "corporate-shareholding-master" }

func (c CorporateShareholdingMasterAPI) BuildURL(symbol string) string {
	// Query the direct equities shareholding master matrix index path
	return fmt.Sprintf("https://www.nseindia.com/api/corporate-share-holdings-master?index=equities&symbol=%s", symbol)
}

func (c CorporateShareholdingMasterAPI) ParseResponse(body io.Reader) ([]UniversalRecord, error) {
	// 1. Structural representation matching the inner targets of the JSON array elements
	type ShareholdingItem struct {
		Date string `json:"date"`
		Xbrl string `json:"xbrl"`
	}

	var items []ShareholdingItem
	bodyBytes, err := io.ReadAll(body)
	if err != nil {
		return nil, fmt.Errorf("failed to read shareholding body pipeline: %w", err)
	}

	if err := json.Unmarshal(bodyBytes, &items); err != nil {
		return nil, fmt.Errorf("failed to deserialize shareholding entries: %w", err)
	}

	var results []UniversalRecord

	// 2. Iterate and process fields to mirror Strategy 10's signature convention
	for _, item := range items {
		if item.Xbrl == "" || !strings.HasPrefix(item.Xbrl, "http") {
			continue
		}

		// Extract base filename from the URL path string
		// E.g., "https://.../SHP_1648419_10042026103603_WEB.xml" -> "SHP_1648419_10042026103603_WEB.xml"
		webFileName := filepath.Base(item.Xbrl)
		ext := filepath.Ext(webFileName)
		
		// Strip trailing extension string (.xml)
		rawToken := webFileName
		if len(ext) > 0 {
			rawToken = webFileName[:len(webFileName)-len(ext)]
		}

		// Clean up spaces in the period text token to maintain pristine path names
		cleanDate := strings.ReplaceAll(item.Date, " ", "-")

		// Reconstruct the uniform label token: "31-MAR-2026_File_SHP_1648419_10042026103603_WEB"
		uniquePeriodLabel := fmt.Sprintf("%s_File_%s", cleanDate, rawToken)

		results = append(results, UniversalRecord{
			Period:      uniquePeriodLabel,
			DownloadURL: item.Xbrl,
		})
	}

	return results, nil
}