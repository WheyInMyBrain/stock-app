use std::path::Path;
use reqwest::header::{ACCEPT, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};

const USER_AGENT_VAL: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalStockMetadata {
    pub symbol: String,
    pub company_name: String,
    pub bse_code: String,
    pub nse_code: String,
}

#[derive(Deserialize)]
struct BseRawRow {
    #[serde(rename = "strSricpCode")]
    str_scrip_code: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
    #[serde(rename = "scripName")]
    scrip_name: Option<String>,
}

#[derive(Deserialize)]
struct NseRawPayload {
    data: Option<Vec<NseRawRow>>,
}

#[derive(Deserialize)]
struct NseRawRow {
    symbol: Option<String>,
    #[serde(rename = "companyName")]
    company_name: Option<String>,
}

#[derive(Deserialize)]
struct NseChartRawPayload {
    data: Option<Vec<NseChartRawRow>>,
}

#[derive(Deserialize)]
struct NseChartRawRow {
    symbol: Option<String>,
    scripcode: Option<String>,
}

pub async fn run_unified_search_and_save(
    nse_client: &reqwest::Client,
    bse_client: &reqwest::Client,
    symbol: &str,
    global_data_dir: &Path,
) -> Result<FinalStockMetadata, String> {
    let target_symbol = symbol.trim().to_uppercase();
    let query_lower = target_symbol.to_lowercase();

    let nse_url = format!("https://www.nseindia.com/api/NextApi/globalSearch/equity?symbol={}", query_lower);
    let bse_url = format!("https://api.bseindia.com/BseIndiaAPI/api/GetQuoteAllSearchDatabeta/w?searchString={}", query_lower);
    let chart_url = format!("https://charting.nseindia.com/v1/exchanges/symbolsDynamic?symbol={}&segment=", target_symbol);

    let nse_task = async {
        if let Ok(res) = nse_client.get(&nse_url)
            .header(USER_AGENT, USER_AGENT_VAL)
            .header(REFERER, "https://www.nseindia.com/")
            .header(ACCEPT, "*/*")
            .send()
            .await 
        {
            if res.status() == reqwest::StatusCode::OK {
                return res.bytes().await.ok();
            }
        }
        None
    };

    let bse_task = async {
        if let Ok(res) = bse_client.get(&bse_url)
            .header(USER_AGENT, USER_AGENT_VAL)
            .header(REFERER, "https://www.bseindia.com/")
            .header(ORIGIN, "https://www.bseindia.com")
            .header(ACCEPT, "application/json, text/plain, */*")
            .send()
            .await 
        {
            if res.status() == reqwest::StatusCode::OK {
                return res.bytes().await.ok();
            }
        }
        None
    };

    let chart_task = async {
        if let Ok(res) = nse_client.get(&chart_url)
            .header(USER_AGENT, USER_AGENT_VAL)
            .header(REFERER, "https://charting.nseindia.com/")
            .header(ACCEPT, "application/json, text/plain, */*")
            .send()
            .await 
        {
            if res.status() == reqwest::StatusCode::OK {
                return res.bytes().await.ok();
            }
        }
        None
    };

    let (nse_bytes, bse_bytes, chart_bytes) = tokio::join!(nse_task, bse_task, chart_task);

    let mut company_name = String::new();
    if let Some(bytes) = nse_bytes {
        if let Ok(payload) = serde_json::from_slice::<NseRawPayload>(&bytes) {
            if let Some(rows) = payload.data {
                for row in rows {
                    if let Some(sym) = row.symbol {
                        if sym.trim().to_uppercase() == target_symbol {
                            company_name = row.company_name.unwrap_or_default();
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut bse_code = String::new();
    if let Some(bytes) = bse_bytes {
        if let Ok(rows) = serde_json::from_slice::<Vec<BseRawRow>>(&bytes) {
            for row in &rows {
                if let Some(ref s_name) = row.short_name {
                    if s_name.trim().to_uppercase() == target_symbol {
                        bse_code = row.str_scrip_code.clone().unwrap_or_default();
                        if company_name.is_empty() {
                            company_name = row.scrip_name.clone().unwrap_or_default();
                        }
                        break;
                    }
                }
            }
        }
    }

    let mut nse_code = String::new();
    if let Some(bytes) = chart_bytes {
        if let Ok(payload) = serde_json::from_slice::<NseChartRawPayload>(&bytes) {
            if let Some(rows) = payload.data {
                let target_chart_sym = format!("{}-EQ", target_symbol);
                for row in rows {
                    if let Some(sym) = row.symbol {
                        let clean_sym = sym.trim().to_uppercase();
                        if clean_sym == target_symbol || clean_sym == target_chart_sym {
                            nse_code = row.scripcode.unwrap_or_default();
                            break;
                        }
                    }
                }
            }
        }
    }

    if company_name.is_empty() {
        company_name = target_symbol.clone();
    }

    let meta = FinalStockMetadata {
        symbol: target_symbol.clone(),
        company_name,
        bse_code,
        nse_code,
    };

    let target_dir = global_data_dir.join(&target_symbol);
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let file_path = target_dir.join("metadata.json");
    let json_bytes = serde_json::to_vec_pretty(&meta)
        .map_err(|e| format!("Serialization error: {}", e))?;

    std::fs::write(&file_path, json_bytes)
        .map_err(|e| format!("Failed to write file to disk: {}", e))?;

    Ok(meta)
}