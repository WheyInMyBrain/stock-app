use std::sync::Mutex;
use tokio::sync::OnceCell;
use downloader::client::{NseClient, BseClient};
use downloader::nse::{execute_nse_strategy, api::NseEndpoint};
use downloader::bse::{execute_bse_strategy, api::BseEndpoint};
use downloader::search::load_stock_metadata;
use crate::commands::data_dir::resolve_data_directory_headless;

#[derive(Clone, Debug)]
pub struct ActiveDownload {
    pub current_api: String,
    pub filename: String,
    pub percentage: f32,
    pub current_step: usize,
    pub total_steps: usize,
}

#[derive(Clone, Debug)]
pub struct IngestionProgress {
    pub ticker: String,
    pub nse_active: bool,
    pub bse_active: bool,
    pub nse_downloads: Vec<ActiveDownload>,
    pub bse_downloads: Vec<ActiveDownload>,
    pub is_done: bool,
    pub current_phase: String,
    pub last_step: usize,
}

pub static ACTIVE_INGESTION: Mutex<Option<IngestionProgress>> = Mutex::new(None);

pub struct ExchangeClients {
    pub nse: NseClient,
    pub bse: BseClient,
}

pub static EXCHANGE_CLIENTS: OnceCell<ExchangeClients> = OnceCell::const_new();

pub async fn initialize_exchange_clients() -> Result<(), String> {
    if EXCHANGE_CLIENTS.get().is_some() {
        return Ok(());
    }

    let mut last_error = String::new();

    for attempt in 1..=3 {
        let nse_task = NseClient::new();
        let bse_task = BseClient::new();

        let (nse_res, bse_res) = tokio::join!(nse_task, bse_task);

        match (nse_res, bse_res) {
            (Ok(nse), Ok(bse)) => {
                let _ = EXCHANGE_CLIENTS.set(ExchangeClients { nse, bse });
                return Ok(());
            }
            (nse_err, bse_err) => {
                let nse_msg = match nse_err {
                    Err(e) => format!("NSE cookie sync failed: {}", e),
                    Ok(_) => "NSE warded successfully".to_string(),
                };
                let bse_msg = match bse_err {
                    Err(e) => format!("BSE cookie sync failed: {}", e),
                    Ok(_) => "BSE warded successfully".to_string(),
                };
                
                last_error = format!("Attempt {}: [{} | {}]", attempt, nse_msg, bse_msg);
                println!("\x1b[96m[downloader] ⚠️ [Exchange Init] {}, retrying...\x1b[0m", last_error);

                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    Err(format!("Exchange clients initialization permanently failed after 3 attempts. Last state: {}", last_error))
}

pub async fn run_all(symbol: &str, nse_run: bool, bse_run: bool) -> Result<(), String> {
    let clients = EXCHANGE_CLIENTS.get().ok_or("Exchange clients not initialized".to_string())?;
    let data_dir = resolve_data_directory_headless();
    let meta = load_stock_metadata(symbol, &data_dir).map_err(|e| format!("Metadata lookup error: {}", e))?;

    let nse_suite = vec![
        NseEndpoint::SymbolCoreData,
        
        NseEndpoint::HistoricalChartData,
        NseEndpoint::CorporateDetails,
        NseEndpoint::CorporateActions,
        NseEndpoint::BulkBlockDeals,
        NseEndpoint::AnnualReports,
        NseEndpoint::AnnualReportsXbrl,
        NseEndpoint::FinancialResults,
        NseEndpoint::CorporateGovernance,
        NseEndpoint::CorporateAnnouncements,
        NseEndpoint::BusinessSustainability,
        NseEndpoint::CorporateBoardMeetings,
        NseEndpoint::InsiderPlan,
        NseEndpoint::InvestorComplaints,
        NseEndpoint::PeerQuarters,
        NseEndpoint::PeerIndices,
        NseEndpoint::PeerComparisonMatrix,
        NseEndpoint::ChartSymbolMetadata,
        NseEndpoint::RealTimeChartSeed,
        NseEndpoint::RealTimeChartDelta(None),
        NseEndpoint::IntegratedFilingResults,
    ];

    let bse_suite = vec![
        BseEndpoint::ChartSymbolMetadata,
        BseEndpoint::CorporateDetailsHeader,
        BseEndpoint::CorporateDetails,
        BseEndpoint::ScripPricingHeader,
        BseEndpoint::LiveTradingTurnover,
        BseEndpoint::PeerValuationMatrix,
        BseEndpoint::CorporateInfoDirectory,
        BseEndpoint::CorporateActions,
        BseEndpoint::ShareholderMeetings,
        BseEndpoint::BoardMeetings,
        BseEndpoint::HistoricalChartData,
        BseEndpoint::BulkDeals,
        BseEndpoint::BlockDeals,
        BseEndpoint::FinancialResults,
        BseEndpoint::VotingResults,
        BseEndpoint::ShareholdingPattern,
        BseEndpoint::CorporateGovernance,
        BseEndpoint::InvestorComplaints,
        BseEndpoint::IntegratedFinanceData,
    ];

    {
        let mut guard = ACTIVE_INGESTION.lock().unwrap();
        *guard = Some(IngestionProgress {
            ticker: symbol.to_string(),
            nse_active: nse_run,
            bse_active: bse_run,
            nse_downloads: Vec::new(),
            bse_downloads: Vec::new(),
            is_done: false,
            current_phase: "Ingestion".to_string(),
            last_step: 0,
        });
    }

    let nse_idx = std::sync::atomic::AtomicUsize::new(0);
    let nse_total = nse_suite.len();
    
    let nse_worker = || async {
        loop {
            let idx = nse_idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if idx >= nse_total { break; }
            let endpoint = nse_suite[idx];
            let current_step = idx + 1;
            
            {
                let mut guard = ACTIVE_INGESTION.lock().unwrap();
                if let Some(ref mut progress) = *guard {
                    progress.nse_downloads.push(ActiveDownload {
                        current_api: endpoint.name().to_string(),
                        filename: String::new(),
                        percentage: 0.0,
                        current_step,
                        total_steps: nse_total,
                    });
                }
            }

            let _ = execute_nse_strategy(&clients.nse, symbol, &meta.nse_code, endpoint, &data_dir, false).await;

            {
                let mut guard = ACTIVE_INGESTION.lock().unwrap();
                if let Some(ref mut progress) = *guard {
                    if let Some(track) = progress.nse_downloads.iter_mut().find(|d| d.current_api == endpoint.name()) {
                        track.percentage = 100.0;
                    }
                }
            }
        }
    };

    let bse_idx = std::sync::atomic::AtomicUsize::new(0);
    let bse_total = bse_suite.len();

    let bse_worker = || async {
        loop {
            let idx = bse_idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if idx >= bse_total { break; }
            let endpoint = bse_suite[idx];
            let current_step = idx + 1;

            {
                let mut guard = ACTIVE_INGESTION.lock().unwrap();
                if let Some(ref mut progress) = *guard {
                    progress.bse_downloads.push(ActiveDownload {
                        current_api: endpoint.name().to_string(),
                        filename: String::new(),
                        percentage: 0.0,
                        current_step,
                        total_steps: bse_total,
                    });
                }
            }

            let _ = execute_bse_strategy(&clients.bse, symbol, &meta.bse_code, endpoint, &data_dir, false).await;

            {
                let mut guard = ACTIVE_INGESTION.lock().unwrap();
                if let Some(ref mut progress) = *guard {
                    if let Some(track) = progress.bse_downloads.iter_mut().find(|d| d.current_api == endpoint.name()) {
                        track.percentage = 100.0;
                    }
                }
            }
        }
    };

    let nse_task = async {
        if nse_run {
            tokio::join!(nse_worker(), nse_worker(), nse_worker(), nse_worker());
        }
    };

    let bse_task = async {
        if bse_run {
            tokio::join!(bse_worker(), bse_worker(), bse_worker(), bse_worker());
        }
    };

    tokio::join!(nse_task, bse_task);

    {
        let mut guard = ACTIVE_INGESTION.lock().unwrap();
        if let Some(ref mut progress) = *guard {
            progress.is_done = true;
        }
    }

    Ok(())
}

pub async fn nse_endpoint_run(symbol: &str, endpoint: NseEndpoint) -> Result<(), String> {
    let clients = EXCHANGE_CLIENTS.get().ok_or("Exchange clients not initialized".to_string())?;
    let data_dir = resolve_data_directory_headless();
    let meta = load_stock_metadata(symbol, &data_dir).map_err(|e| format!("Metadata lookup error: {}", e))?;

    execute_nse_strategy(&clients.nse, symbol, &meta.nse_code, endpoint, &data_dir, false).await
}

pub async fn bse_endpoint_run(symbol: &str, endpoint: BseEndpoint) -> Result<(), String> {
    let clients = EXCHANGE_CLIENTS.get().ok_or("Exchange clients not initialized".to_string())?;
    let data_dir = resolve_data_directory_headless();
    let meta = load_stock_metadata(symbol, &data_dir).map_err(|e| format!("Metadata lookup error: {}", e))?;

    execute_bse_strategy(&clients.bse, symbol, &meta.bse_code, endpoint, &data_dir, false).await
}

pub fn initialize_go_daemon() {}

pub async fn run_sidecar_downloader(
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    let flags = extra_args.ok_or_else(|| "Missing execution flags".to_string())?;
    run_sidecar_downloader_native(flags).await
}

pub async fn run_sidecar_downloader_native(_extra_args: Vec<String>) -> Result<String, String> {
    Ok("Signal received".to_string())
}