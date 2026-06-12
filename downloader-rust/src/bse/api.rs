use std::path::Path;
use chrono::Datelike;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BseEndpoint {
    ChartSymbolMetadata,
    CorporateDetailsHeader,
    CorporateDetails,
    ScripPricingHeader,
    LiveTradingTurnover,
    PeerValuationMatrix,
    CorporateInfoDirectory,
    CorporateActions,
    ShareholderMeetings,
    BoardMeetings,
    HistoricalChartData,
    BulkDeals,
    BlockDeals,
    FinancialResults,
    VotingResults,
    ShareholdingPattern,
    CorporateGovernance,
    InvestorComplaints,
    IntegratedFinanceData,
    SensexHistoricalData,
}

impl BseEndpoint {
    pub fn name(&self) -> &'static str {
        match self {
            BseEndpoint::ChartSymbolMetadata => "chart-symbol-metadata",
            BseEndpoint::CorporateDetailsHeader => "corporate-details-header",
            BseEndpoint::CorporateDetails => "corporate-details",
            BseEndpoint::ScripPricingHeader => "scrip-pricing-header",
            BseEndpoint::LiveTradingTurnover => "live-trading-turnover",
            BseEndpoint::PeerValuationMatrix => "peer-valuation-matrix",
            BseEndpoint::CorporateInfoDirectory => "corporate-info-directory",
            BseEndpoint::CorporateActions => "corporate-actions",
            BseEndpoint::ShareholderMeetings => "shareholder-meetings",
            BseEndpoint::BoardMeetings => "board-meetings",
            BseEndpoint::HistoricalChartData => "historical-chart-data",
            BseEndpoint::BulkDeals => "bulk-deals",
            BseEndpoint::BlockDeals => "block-deals",
            BseEndpoint::FinancialResults => "financial-results-docs",
            BseEndpoint::VotingResults => "voting-results-docs",
            BseEndpoint::ShareholdingPattern => "shareholding-pattern-docs",
            BseEndpoint::CorporateGovernance => "corporate-governance-docs",
            BseEndpoint::InvestorComplaints => "investor-complaints-docs",
            BseEndpoint::IntegratedFinanceData => "integrated-finance-data",
            BseEndpoint::SensexHistoricalData => "sensex-historical-data",
        }
    }

    pub fn build_url(&self, symbol: &str, bse_code: &str, _global_data_dir: &Path) -> String {
        match self {
            BseEndpoint::ChartSymbolMetadata => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/ListScripSmartSearch_ng/w?searchString={}", symbol)
            }
            BseEndpoint::CorporateDetailsHeader => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/ComHeadernew/w?quotetype=&scripcode={}&seriesid=", bse_code)
            }
            BseEndpoint::CorporateDetails => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/StockTrading/w?flag=&quotetype=EQ&scripcode={}", bse_code)
            }
            BseEndpoint::ScripPricingHeader => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/getScripHeaderData/w?Debtflag=&scripcode={}&seriesid=", bse_code)
            }
            BseEndpoint::LiveTradingTurnover => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/StockTrading/w?flag=&quotetype=EQ&scripcode={}", bse_code)
            }
            BseEndpoint::PeerValuationMatrix => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/PeerGpCom/w?scripcode={}&scripcomare=", bse_code)
            }
            BseEndpoint::CorporateInfoDirectory => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/CorpInfoNew/w?scripcode={}", bse_code)
            }
            BseEndpoint::CorporateActions => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/CorporateAction/w?scripcode={}", bse_code)
            }
            BseEndpoint::ShareholderMeetings => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/ShareHolderMeeting/w?scripcode={}", bse_code)
            }
            BseEndpoint::BoardMeetings => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/BoardMeeting/w?scripcode={}", bse_code)
            }
            BseEndpoint::HistoricalChartData => {
                let now = chrono::Local::now();
                let from_date_str = format!("{}0101", now.year() - 10);
                let to_date_str = now.format("%Y%m%d").to_string();
                format!(
                    "https://api.bseindia.com/BseIndiaAPI/api/StockReachGraph/w?scripcode={}&flag=1&fromdate={}&todate={}&seriesid=",
                    bse_code, from_date_str, to_date_str
                )
            }
            BseEndpoint::BulkDeals => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/BulkblockDeal/w?fromdt=&todt=&type=1&scripcode={}", bse_code)
            }
            BseEndpoint::BlockDeals => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/BulkblockDeal/w?fromdt=&todt=&type=2&scripcode={}", bse_code)
            }
            BseEndpoint::FinancialResults => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/Result_Arch_ng/w?scrip_cd={}", bse_code)
            }
            BseEndpoint::VotingResults => {
                let now = chrono::Local::now();
                let from_dt = format!("{:02}/{:02}/{}", now.day(), now.month(), now.year() - 2);
                let to_dt = now.format("%d/%m/%Y").to_string();
                format!("https://api.bseindia.com/BseIndiaAPI/api/VotingResults/w?fromdt={}&todt={}&type=0&scripcode={}", from_dt, to_dt, bse_code)
            }
            BseEndpoint::ShareholdingPattern => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/SHPQNewFormat/w?scripcode={}", bse_code)
            }
            BseEndpoint::CorporateGovernance => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/CGArchivewise/w?scripcode={}", bse_code)
            }
            BseEndpoint::InvestorComplaints => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/XbrlInvestorComplaint/w?scripcode={}", bse_code)
            }
            BseEndpoint::IntegratedFinanceData => {
                format!("https://api.bseindia.com/BseIndiaAPI/api/Integratedfinancedata/w?scripcode={}", bse_code)
            }
            BseEndpoint::SensexHistoricalData => {
                let now = chrono::Local::now();
                let from_date_str = format!("{}0101", now.year() - 10);
                let to_date_str = now.format("%Y%m%d").to_string();
                format!(
                    "https://api.bseindia.com/BseIndiaAPI/api/SensexGraphData/w?index=98&flag=1&sector=&seriesid=DT&frd={}&tod={}&ext=.json",
                    from_date_str, to_date_str
                )
            }
        }
    }

    pub fn parse_attachments(
        &self,
        symbol: &str,
        global_data_dir: &Path,
        raw_json_bytes: &[u8],
    ) -> Vec<(String, String)> {
        let mut results = Vec::new();

        match self {
            BseEndpoint::HistoricalChartData => {
                #[derive(serde::Deserialize)]
                struct RawBseChartResponse {
                    #[serde(rename = "Data")]
                    data: Option<String>,
                }
                #[derive(serde::Deserialize)]
                struct RawBseDataRow {
                    dttm: Option<String>,
                    vale1: Option<String>,
                    vole: Option<String>,
                }
                #[derive(serde::Serialize)]
                struct NormalizedBseStructure {
                    #[serde(rename = "grapthData")]
                    grapth_data: Vec<(i64, f64, f64)>,
                }

                if let Ok(raw_resp) = serde_json::from_slice::<RawBseChartResponse>(raw_json_bytes) {
                    if let Some(data_str) = raw_resp.data {
                        if let Ok(rows) = serde_json::from_str::<Vec<RawBseDataRow>>(&data_str) {
                            let mut normalized_data = Vec::new();
                            for row in rows {
                                if let Some(dttm_str) = row.dttm {
                                    if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(dttm_str.trim(), "%a %b %d %Y %H:%M:%S") {
                                        let unix_millis = naive_dt.and_utc().timestamp_millis();
                                        let price_float: f64 = row.vale1.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
                                        let volume_float: f64 = row.vole.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
                                        normalized_data.push((unix_millis, price_float, volume_float));
                                    }
                                }
                            }
                            let final_struct = NormalizedBseStructure { grapth_data: normalized_data };
                            let output_dir = global_data_dir.join(symbol).join("bse_historical-chart-data");
                            let target_path = output_dir.join("10Y.json");
                            if let Ok(json_bytes) = serde_json::to_vec_pretty(&final_struct) {
                                let _ = std::fs::write(target_path, json_bytes);
                            }
                        }
                    }
                }
            }
            BseEndpoint::FinancialResults => {
                #[derive(serde::Deserialize)]
                struct BseResultsPayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseResultRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseResultRow {
                    #[serde(rename = "Quarter")]
                    quarter: Option<String>,
                    stand_xbrl_link: Option<serde_json::Value>,
                    conso_xbrl_link: Option<serde_json::Value>,
                    #[serde(rename = "Weburl")]
                    web_url: Option<String>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseResultsPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(q) = row.quarter {
                                let clean_quarter = q.replace(";", "_").replace(":", "-").replace("/", "-");

                                if let Some(web_url) = row.web_url {
                                    let clean_url = web_url.trim();
                                    if !clean_url.is_empty() && !clean_url.to_lowercase().ends_with(".pdf") {
                                        results.push((format!("{}_WebReport", clean_quarter), clean_url.to_string()));
                                    }
                                }
                                if let Some(stand_val) = row.stand_xbrl_link {
                                    if let Some(stand_path) = stand_val.as_str() {
                                        let clean_path = stand_path.trim();
                                        if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                            results.push((format!("{}_Standalone_XBRL", clean_quarter), format!("https://www.bseindia.com{}", clean_path)));
                                        }
                                    }
                                }
                                if let Some(conso_val) = row.conso_xbrl_link {
                                    if let Some(conso_path) = conso_val.as_str() {
                                        let clean_path = conso_path.trim();
                                        if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                            results.push((format!("{}_Consolidated_XBRL", clean_quarter), format!("https://www.bseindia.com{}", clean_path)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BseEndpoint::VotingResults => {
                #[derive(serde::Deserialize)]
                struct BseVotingPayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseVotingRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseVotingRow {
                    #[serde(rename = "Fld_MasterID")]
                    fld_master_id: Option<serde_json::Value>,
                    #[serde(rename = "Description")]
                    description: Option<String>,
                    #[serde(rename = "fld_srno")]
                    fld_srno: Option<serde_json::Value>,
                    #[serde(rename = "Fld_XMLName")]
                    fld_xml_name: Option<serde_json::Value>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseVotingPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(xml_val) = row.fld_xml_name {
                                if let Some(xml_path) = xml_val.as_str() {
                                    let clean_path = xml_path.trim();
                                    if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                        let desc = row.description.unwrap_or_else(|| "Unknown".to_string()).replace(" ", "_");
                                        let master_id = match row.fld_master_id {
                                            Some(serde_json::Value::Number(n)) => n.to_string(),
                                            Some(serde_json::Value::String(s)) => s,
                                            _ => "0".to_string(),
                                        };
                                        let sr_no = match row.fld_srno {
                                            Some(serde_json::Value::Number(n)) => n.to_string(),
                                            Some(serde_json::Value::String(s)) => s,
                                            _ => "0".to_string(),
                                        };

                                        let base_name = format!("ID_{}_{}_SrNo_{}", master_id, desc, sr_no);
                                        results.push((format!("{}_DataLedger", base_name), format!("https://www.bseindia.com{}", clean_path)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BseEndpoint::ShareholdingPattern => {
                #[derive(serde::Deserialize)]
                struct BseShpPayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseShpRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseShpRow {
                    yr: Option<String>,
                    qtr: Option<String>,
                    status: Option<String>,
                    xbrlurl: Option<serde_json::Value>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseShpPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(xbrl_val) = row.xbrlurl {
                                if let Some(path_str) = xbrl_val.as_str() {
                                    let clean_path = path_str.trim();
                                    if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                        let clean_qtr = row.qtr.unwrap_or_default().replace(" ", "_");
                                        let clean_yr = row.yr.unwrap_or_default().replace(" ", "").replace("-", "_");
                                        let status = row.status.unwrap_or_default();
                                        let local_token = format!("SHP_{}_{}_{}", clean_qtr, clean_yr, status);
                                        results.push((local_token, format!("https://www.bseindia.com{}", clean_path)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BseEndpoint::CorporateGovernance => {
                #[derive(serde::Deserialize)]
                struct BseCgPayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseCgRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseCgRow {
                    #[serde(rename = "Fld_QuarterId")]
                    fld_quarter_id: Option<serde_json::Value>,
                    #[serde(rename = "Year")]
                    year: Option<String>,
                    qtr: Option<String>,
                    status: Option<String>,
                    xbrlurl: Option<serde_json::Value>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseCgPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(xbrl_val) = row.xbrlurl {
                                if let Some(path_str) = xbrl_val.as_str() {
                                    let clean_path = path_str.trim();
                                    if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                        let clean_qtr = row.qtr.unwrap_or_default().replace(" ", "_");
                                        let clean_yr = row.year.unwrap_or_default().replace(" ", "").replace("-", "_");
                                        let status = row.status.unwrap_or_default();
                                        let qtr_id_str = match row.fld_quarter_id {
                                            Some(serde_json::Value::Number(n)) => n.to_string(),
                                            Some(serde_json::Value::String(s)) => s,
                                            _ => "".to_string(),
                                        };
                                        let local_token = format!("CG_{}_{}_ID_{}_{}", clean_qtr, clean_yr, qtr_id_str, status);
                                        results.push((local_token, format!("https://www.bseindia.com{}", clean_path)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BseEndpoint::InvestorComplaints => {
                #[derive(serde::Deserialize)]
                struct BseComplaintsPayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseComplaintsRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseComplaintsRow {
                    yr: Option<String>,
                    qtr: Option<String>,
                    qtrid: Option<serde_json::Value>,
                    status: Option<String>,
                    xbrlurl: Option<serde_json::Value>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseComplaintsPayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(xbrl_val) = row.xbrlurl {
                                if let Some(path_str) = xbrl_val.as_str() {
                                    let clean_path = path_str.trim();
                                    if !clean_path.is_empty() && !clean_path.to_lowercase().ends_with(".pdf") {
                                        let clean_qtr = row.qtr.unwrap_or_default().replace(" ", "_");
                                        let clean_yr = row.yr.unwrap_or_default().replace(" ", "").replace("-", "_");
                                        let status = row.status.unwrap_or_default();
                                        let qtr_id_str = match row.qtrid {
                                            Some(serde_json::Value::Number(n)) => n.to_string(),
                                            Some(serde_json::Value::String(s)) => s,
                                            _ => "".to_string(),
                                        };
                                        let local_token = format!("Complaints_{}_{}_ID_{}_{}", clean_qtr, clean_yr, qtr_id_str, status);
                                        results.push((local_token, format!("https://www.bseindia.com{}", clean_path)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BseEndpoint::IntegratedFinanceData => {
                #[derive(serde::Deserialize)]
                struct BseFinancePayload {
                    #[serde(rename = "Table")]
                    table: Option<Vec<BseFinanceRow>>,
                }
                #[derive(serde::Deserialize)]
                struct BseFinanceRow {
                    #[serde(rename = "Yr")]
                    yr: Option<serde_json::Value>,
                    #[serde(rename = "Quarter_Name")]
                    quarter_name: Option<String>,
                    #[serde(rename = "xbrlurl")]
                    xbrlurl: Option<String>,
                }

                if let Ok(payload) = serde_json::from_slice::<BseFinancePayload>(raw_json_bytes) {
                    if let Some(rows) = payload.table {
                        for row in rows {
                            if let Some(url_path) = row.xbrlurl {
                                let clean_path = url_path.trim();
                                if !clean_path.is_empty() {
                                    let year_str = match row.yr {
                                        Some(serde_json::Value::Number(n)) => n.to_string(),
                                        Some(serde_json::Value::String(s)) => s,
                                        _ => "0".to_string(),
                                    };
                                    let q_name = row.quarter_name
                                        .unwrap_or_else(|| "Statement".to_string())
                                        .replace(' ', "_");

                                    let filename = format!("Year_{}_{}_XBRLDoc", year_str, q_name);
                                    let download_url = format!("https://www.bseindia.com{}", clean_path);
                                    results.push((filename, download_url));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        results
    }
}