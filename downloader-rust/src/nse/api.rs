// downloader-rust/src/nse/api.rs
use chrono::Datelike;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NseEndpoint {
    SymbolCoreData,
    HistoricalChartData,
    CorporateActions,
    BulkBlockDeals,
    AnnualReports,
    AnnualReportsXbrl,
    FinancialResults, 
    CorporateGovernance,
    CorporateAnnouncements,
    BusinessSustainability,
    CorporateBoardMeetings,
    InsiderPlan,
    InvestorComplaints,
    PeerQuarters,
    PeerIndices,
    PeerComparisonMatrix,
    ChartSymbolMetadata,
    RealTimeChartSeed,
    RealTimeChartDelta(Option<i64>),
}

// === Annual Reports Schemas ===
#[derive(Deserialize)]
struct NseAnnualReportRow {
    #[serde(rename = "fromYr")]
    from_yr: Option<String>,
    #[serde(rename = "toYr")]
    to_yr: Option<String>,
    #[serde(rename = "fileName")]
    file_name: Option<String>,
}

#[derive(Deserialize)]
struct NseAnnualReportPayload {
    data: Option<Vec<NseAnnualReportRow>>,
}

// === Financial Results Schemas ===
#[derive(Deserialize)]
struct NseFinancialRow {
    #[serde(rename = "toDate")]
    to_date: Option<String>,
    xbrl: Option<String>,
    consolidated: Option<String>,
}

// === Corporate Governance Schemas ===
#[derive(Deserialize)]
struct NseGovernanceRow {
    date: Option<String>,
    xbrl: Option<String>,
}

#[derive(Deserialize)]
struct NseGovernancePayload {
    data: Option<Vec<NseGovernanceRow>>,
}

// === Corporate Announcements Schemas ===
#[derive(Deserialize)]
struct NseAnnouncementRow {
    #[serde(rename = "seq_id")]
    seq_id: Option<String>,
    #[serde(rename = "attchmntFile")]
    attachment: Option<String>,
    #[serde(rename = "an_dt")]
    announcement: Option<String>,
}

// === Business Sustainability Schemas ===
#[derive(Deserialize)]
struct NseSustainabilityRow {
    #[serde(rename = "fyFrom")]
    fy_from: Option<i32>,
    #[serde(rename = "fyTo")]
    fy_to: Option<i32>,
    #[serde(rename = "xbrlFile")]
    xbrl_file: Option<String>,
}

#[derive(Deserialize)]
struct NseSustainabilityPayload {
    data: Option<Vec<NseSustainabilityRow>>,
}

// === Corporate Board Meetings Schemas ===
#[derive(Deserialize)]
struct NseBoardMeetingRow {
    #[serde(rename = "bm_date")]
    meeting_date: Option<String>,
    #[serde(rename = "bm_timestamp")]
    timestamp: Option<String>,
    attachment: Option<String>,
    ixbrl: Option<String>,
}

// === Insider Trading Plans Schemas ===
#[derive(Deserialize)]
struct NseInsiderPlanRow {
    #[serde(rename = "appid")]
    app_id: Option<String>,
    #[serde(rename = "submissionDate")]
    submission_date: Option<String>,
    attachment: Option<String>,
    ixbrl: Option<String>,
}

// === Investor Complaints Schemas ===
#[derive(Deserialize)]
struct NseComplaintRow {
    date: Option<String>,
    xbrl: Option<String>,
}

#[derive(Deserialize)]
struct NseComplaintPayload {
    data: Option<Vec<NseComplaintRow>>,
}

// === Peer Comparison Schemas ===
#[derive(Deserialize)]
struct NseQuarterResponse {
    value: Option<String>,
}

impl NseEndpoint {
    /// Returns the naming string token used for dynamic directory folder tree maps
    pub fn name(&self) -> &'static str {
        match self {
            NseEndpoint::SymbolCoreData => "symbol-core-data",
            NseEndpoint::HistoricalChartData => "historical-chart-data",
            NseEndpoint::CorporateActions => "corporates-corporateActions",
            NseEndpoint::BulkBlockDeals => "bulk-block-deals",
            NseEndpoint::AnnualReports => "annual-reports",
            NseEndpoint::AnnualReportsXbrl => "annual-reports-xbrl",
            NseEndpoint::FinancialResults => "corporates-financial-results",
            NseEndpoint::CorporateGovernance => "corporate-governance-master",
            NseEndpoint::CorporateAnnouncements => "corporate-announcements",
            NseEndpoint::BusinessSustainability => "corporate-bussiness-sustainabilitiy",
            NseEndpoint::CorporateBoardMeetings => "corporate-board-meetings",
            NseEndpoint::InsiderPlan => "corporate-insider-plan",
            NseEndpoint::InvestorComplaints => "investor-complaints",
            NseEndpoint::PeerQuarters => "peer-quarters",
            NseEndpoint::PeerIndices => "peer-indices",
            NseEndpoint::PeerComparisonMatrix => "peer-comparison-matrix",
            NseEndpoint::ChartSymbolMetadata => "chart-symbol-metadata",
            NseEndpoint::RealTimeChartSeed => "real-time-chart",
            NseEndpoint::RealTimeChartDelta(_) => "real-time-chart-delta",
        }
    }

    /// Forms the physical remote endpoint target URL mapping using target ticker symbols
    pub fn build_url(&self, symbol: &str, nse_code: &str, global_data_dir: &std::path::Path) -> String {
        match self {
            NseEndpoint::SymbolCoreData => {
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getSymbolData&marketType=N&series=EQ&symbol={}", symbol)
            }
            NseEndpoint::HistoricalChartData => {
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getSymbolChartData&symbol={}EQN&days=30Y", symbol)
            }
            NseEndpoint::CorporateActions => {
                format!("https://www.nseindia.com/api/corporates-corporateActions?index=equities&symbol={}", symbol)
            }
            NseEndpoint::BulkBlockDeals => {
                let now = chrono::Local::now();
                let from_date_str = format!("{:02}-{:02}-{}", now.day(), now.month(), now.year() - 30);
                let to_date_str = now.format("%d-%m-%Y").to_string();
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getHistoricalBulkAndBlockData&symbol={}&fromDate={}&toDate={}", symbol, from_date_str, to_date_str)
            }
            NseEndpoint::AnnualReports => {
                format!("https://www.nseindia.com/api/annual-reports?index=equities&symbol={}", symbol)
            }
            NseEndpoint::AnnualReportsXbrl => {
                format!("https://www.nseindia.com/api/annual-reports-xbrl?index=equities&symbol={}", symbol)
            }
            NseEndpoint::FinancialResults => {
                format!("https://www.nseindia.com/api/corporates-financial-results?index=equities&symbol={}&period=Quarterly", symbol)
            }
            NseEndpoint::CorporateGovernance => {
                format!("https://www.nseindia.com/api/corporate-governance-master?index=equities&symbol={}", symbol)
            }
            NseEndpoint::CorporateAnnouncements => {
                format!("https://www.nseindia.com/api/corporate-announcements?index=equities&symbol={}&reqXbrl=false", symbol)
            }
            NseEndpoint::BusinessSustainability => {
                format!("https://www.nseindia.com/api/corporate-bussiness-sustainabilitiy?index=equities&symbol={}", symbol)
            }
            NseEndpoint::CorporateBoardMeetings => {
                format!("https://www.nseindia.com/api/corporate-board-meetings?index=equities&symbol={}", symbol)
            }
            NseEndpoint::InsiderPlan => {
                format!("https://www.nseindia.com/api/corporate-insider-plan?index=equities&symbol={}", symbol)
            }
            NseEndpoint::InvestorComplaints => {
                format!("https://www.nseindia.com/api/investor-complaints?index=equities&symbol={}", symbol)
            }
            NseEndpoint::PeerQuarters => {
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonQuaters&symbol={}", symbol)
            }
            NseEndpoint::PeerIndices => {
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getIndexList&symbol={}", symbol)
            }
            NseEndpoint::PeerComparisonMatrix => {
                format!("https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonQuaters&symbol={}", symbol)
            }
            NseEndpoint::ChartSymbolMetadata => { 
                format!("https://charting.nseindia.com/v1/exchanges/symbolsDynamic?symbol={}-EQ&segment=", symbol)
            }
            NseEndpoint::RealTimeChartSeed => {
                let now_ts = chrono::Utc::now().timestamp();
                format!(
                    "https://charting.nseindia.com/v1/charts/symbolHistoricalData?token={}&fromDate=0&toDate={}&symbol={}-EQ&symbolType=Equity&chartType=I&timeInterval=1",
                    nse_code, now_ts, symbol
                )
            }
            NseEndpoint::RealTimeChartDelta(custom_from_ts) => {
                let from_ts = match *custom_from_ts {
                    Some(ts) => ts,
                    None => {
                        let target_path = global_data_dir.join(symbol).join("nse_real-time-chart").join("endpoint-metadata.json");

                        if let Ok(content) = std::fs::read_to_string(&target_path) {
                            #[derive(serde::Deserialize)]
                            struct SeedRow { time: Option<i64> }
                            #[derive(serde::Deserialize)]
                            struct SeedPayload { data: Option<Vec<SeedRow>> }
                            
                            if let Ok(payload) = serde_json::from_str::<SeedPayload>(&content) {
                                payload.data
                                    .unwrap_or_default()
                                    .iter()
                                    .filter_map(|r| r.time)
                                    .max()
                                    .map(|ms| ms / 1000)
                                    .unwrap_or(0)
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    }
                };

                let now_ts = chrono::Utc::now().timestamp();
                format!(
                    "https://charting.nseindia.com/v1/charts/symbolHistoricalData?fromDate={}&toDate={}&symbol={}-EQ&token={}&symbolType=Equity&chartType=I&timeInterval=1",
                    from_ts, now_ts, symbol, nse_code
                )
            }
        }
    }

    /// Parses internal response payload bytes to extract attachment descriptions: Vec<(period, download_url)>
    pub fn parse_attachments(&self, symbol: &str, global_data_dir: &std::path::Path, raw_json_bytes: &[u8]) -> Vec<(String, String)> {
        let mut results = Vec::new();
        
        match self {
            NseEndpoint::AnnualReports | NseEndpoint::AnnualReportsXbrl => {
                if let Ok(payload) = serde_json::from_slice::<NseAnnualReportPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.data {
                        for row in rows {
                            if let Some(url) = row.file_name {
                                if url.trim().len() > 8 && url.starts_with("http") {
                                    let from = row.from_yr.unwrap_or_else(|| "Unknown".to_string());
                                    let to = row.to_yr.unwrap_or_else(|| "Unknown".to_string());
                                    let period = format!("{}-{}", from, to);
                                    results.push((period, url));
                                }
                            }
                        }
                    }
                }
            }
            NseEndpoint::FinancialResults => {
                if let Ok(rows) = serde_json::from_slice::<Vec<NseFinancialRow>>(raw_json_bytes) {
                    for row in rows {
                        if let Some(url) = row.xbrl {
                            if url.trim().len() > 8 && url.starts_with("http") {
                                let to_date = row.to_date.unwrap_or_else(|| "Unknown".to_string());
                                let file_name_suffix = match row.consolidated.as_deref() {
                                    Some("Consolidated") => "Consolidated",
                                    _ => "Standalone",
                                };
                                let unique_period_name = format!("{}_{}", to_date, file_name_suffix);
                                results.push((unique_period_name, url));
                            }
                        }
                    }
                }
            }
            NseEndpoint::CorporateGovernance => {
                if let Ok(payload) = serde_json::from_slice::<NseGovernancePayload>(raw_json_bytes) {
                    if let Some(rows) = payload.data {
                        for row in rows {
                            if let Some(url) = row.xbrl {
                                if url.trim().len() > 8 && url.starts_with("http") {
                                    let date = row.date.unwrap_or_else(|| "Unknown".to_string());
                                    results.push((date, url));
                                }
                            }
                        }
                    }
                }
            }
            NseEndpoint::CorporateAnnouncements => {
                if let Ok(rows) = serde_json::from_slice::<Vec<NseAnnouncementRow>>(raw_json_bytes) {
                    for row in rows {
                        if let Some(url) = row.attachment {
                            if url.trim().len() > 8 && url.starts_with("http") {
                                let announcement = row.announcement.unwrap_or_else(|| "Unknown".to_string());
                                let clean_date: String = announcement.chars().map(|c| match c {
                                    ':' => '-',
                                    ' ' => '_',
                                    _ => c
                                }).collect();
                                let seq_id = row.seq_id.unwrap_or_else(|| "0".to_string());
                                let unique_file_name = format!("{}_ID-{}", clean_date, seq_id);
                                results.push((unique_file_name, url));
                            }
                        }
                    }
                }
            }
            NseEndpoint::BusinessSustainability => {
                if let Ok(payload) = serde_json::from_slice::<NseSustainabilityPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.data {
                        for row in rows {
                            if let Some(url) = row.xbrl_file {
                                if url.trim().len() > 8 && url.starts_with("http") {
                                    let from = row.fy_from.unwrap_or(0);
                                    let to = row.fy_to.unwrap_or(0);
                                    let base_period = format!("{}-{}", from, to);
                                    results.push((format!("{}_XBRL", base_period), url));
                                }
                            }
                        }
                    }
                }
            }
            NseEndpoint::CorporateBoardMeetings => {
                if let Ok(rows) = serde_json::from_slice::<Vec<NseBoardMeetingRow>>(raw_json_bytes) {
                    for row in rows {
                        let meeting_date = row.meeting_date.unwrap_or_else(|| "Unknown".to_string());
                        let timestamp = row.timestamp.unwrap_or_else(|| "Unknown".to_string());
                        let clean_timestamp: String = timestamp.chars().map(|c| match c {
                            ':' => '-',
                            ' ' => '_',
                            _ => c
                        }).collect();
                        
                        let base_name = format!("Meeting_{}_Filed_{}", meeting_date, clean_timestamp);

                        if let Some(url) = row.attachment {
                            if url.trim().len() > 8 && url.starts_with("http") && url.to_lowercase().ends_with(".xml") {
                                results.push((format!("{}_XBRL", base_name), url));
                            }
                        }

                        // Process the non-PDF interactive HTML layout
                        if let Some(url) = row.ixbrl {
                            if url.trim().len() > 8 && url.starts_with("http") {
                                results.push((format!("{}_iXBRL", base_name), url));
                            }
                        }
                    }
                }
            }
            NseEndpoint::InsiderPlan => {
                if let Ok(rows) = serde_json::from_slice::<Vec<NseInsiderPlanRow>>(raw_json_bytes) {
                    for row in rows {
                        let submission_date = row.submission_date.unwrap_or_else(|| "Unknown".to_string());
                        let clean_date: String = submission_date.chars().map(|c| match c {
                            ':' => '-',
                            ' ' => '_',
                            _ => c
                        }).collect();
                        let app_id = row.app_id.unwrap_or_else(|| "0".to_string());
                        let base_name = format!("Plan_{}_App-{}", clean_date, app_id);

                        if let Some(url) = row.attachment {
                            if url.trim().len() > 8 && url.starts_with("http") {
                                results.push((format!("{}_XBRL", base_name), url));
                            }
                        }
                        if let Some(url) = row.ixbrl {
                            if url.trim().len() > 8 && url.starts_with("http") {
                                results.push((format!("{}_iXBRL", base_name), url));
                            }
                        }
                    }
                }
            }
            NseEndpoint::InvestorComplaints => {
                if let Ok(payload) = serde_json::from_slice::<NseComplaintPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.data {
                        for row in rows {
                            if let Some(url) = row.xbrl {
                                if url.trim().len() > 8 && url.starts_with("http") {
                                    let date = row.date.unwrap_or_else(|| "Unknown".to_string());
                                    
                                    // Extract the URL filename stem cleanly and remove the extension
                                    let raw_token = url.split('/')
                                        .last()
                                        .unwrap_or("")
                                        .split('.')
                                        .next()
                                        .unwrap_or("");
                                        
                                    let unique_period_label = format!("{}_File_{}", date, raw_token);
                                    results.push((unique_period_label, url));
                                }
                            }
                        }
                    }
                }
            }
            NseEndpoint::PeerComparisonMatrix => {
                if let Ok(quarters) = serde_json::from_slice::<Vec<NseQuarterResponse>>(raw_json_bytes) {
                    let mut quarter_list = Vec::new();
                    for q in quarters {
                        if let Some(val) = q.value {
                            if !val.trim().is_empty() {
                                quarter_list.push(val);
                            }
                        }
                    }

                    let indices_path = global_data_dir
                        .join(symbol)
                        .join("nse_peer-indices")
                        .join("endpoint-metadata.json");

                    let indices: Vec<String> = if let Ok(content) = std::fs::read_to_string(&indices_path) {
                        serde_json::from_str(&content).unwrap_or_else(|_| vec!["NIFTY TOTAL MARKET".to_string()])
                    } else {
                        vec!["NIFTY TOTAL MARKET".to_string()]
                    };

                    for q in quarter_list {
                        // 1. Core Industry Combo URL
                        let ind_url = format!(
                            "https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonData&symbol={}&type=C&quarter={}&param=industry&index=&ext=.json",
                            symbol, q
                        );
                        results.push((format!("Industry_{}", q), ind_url));

                        // 2. Specific Index Combo URLs
                        for idx in &indices {
                            let escaped_index = idx.replace(" ", "%20");
                            let clean_index_name = idx.replace(" ", "_");
                            let idx_url = format!(
                                "https://www.nseindia.com/api/NextApi/apiClient/GetQuoteApi?functionName=getPeerComparisonData&symbol={}&type=C&quarter={}&param=index&index={}&ext=.json",
                                symbol, q, escaped_index
                            );
                            results.push((format!("Index_{}_{}", clean_index_name, q), idx_url));
                        }
                    }
                }
            }
            _ => {}
        }
        
        results
    }
}