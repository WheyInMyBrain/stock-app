use crate::commands::memory_pool::store_parsed_table;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct OverviewMetadata {
    pub macro_category: String,
    pub sector: String,
    pub industry: String,
}

pub fn hydrate_overview_metadata(ticker: &str) -> Result<(), String> {
    let loader = crate::database::WorkspaceDataLoader::bind(ticker);
    let mut meta = OverviewMetadata::default();

    let nse_res = loader.load_json_struct::<serde_json::Value>("nse_symbol-core-data/endpoint-metadata");
    let bse_res = loader.load_json_struct::<serde_json::Value>("bse_corporate-details-header/endpoint-metadata");

    if let Ok(nse) = nse_res {
        let sec = &nse["equityResponse"][0]["secInfo"];
        meta.macro_category = sec["macro"].as_str().unwrap_or("").to_string();
        meta.sector = sec["sector"].as_str().unwrap_or("").to_string();
        meta.industry = sec["industryInfo"].as_str().unwrap_or("").to_string();
    } else if let Ok(bse) = bse_res {
        meta.macro_category = bse["Sector"].as_str().unwrap_or("").to_string();
        meta.sector = bse["IndustryNew"].as_str().unwrap_or("").to_string();
        meta.industry = bse["IGroup"].as_str().unwrap_or("").to_string();
    }

    store_parsed_table("overview_metadata", meta);
    Ok(())
}