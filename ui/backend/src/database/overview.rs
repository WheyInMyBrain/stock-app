use crate::commands::memory_pool::store_parsed_table;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DirectorRow {
    pub name: String,
    pub designation: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct OverviewMetadata {
    pub macro_category: String,
    pub sector: String,
    pub industry: String,
    pub isin: String,
    pub bse_code: String,
    pub nse_code: String,
    pub face_value: String,
    pub indexes: Vec<String>,
    pub nse_listing_date: String,
    pub bse_listing_date: String,
    pub address: String,
    pub telephone: String,
    pub fax: String,
    pub email: String,
    pub website: String,
    pub directors: Vec<DirectorRow>,
}

pub fn hydrate_overview_metadata(ticker: &str) -> Result<(), String> {
    let loader = crate::database::WorkspaceDataLoader::bind(ticker);
    let mut meta = OverviewMetadata::default();

    let nse_core = loader.load_json_struct::<serde_json::Value>("nse_symbol-core-data/endpoint-metadata");
    let bse_header = loader.load_json_struct::<serde_json::Value>("bse_corporate-details-header/endpoint-metadata");
    let nse_corp = loader.load_json_struct::<serde_json::Value>("nse_corporate-details/endpoint-metadata");
    let bse_directory = loader.load_json_struct::<serde_json::Value>("bse_corporate-info-directory/endpoint-metadata");

    if let Ok(nse) = nse_core {
        let root = &nse["equityResponse"][0];
        let sec = &root["secInfo"];
        meta.macro_category = sec["macro"].as_str().unwrap_or("").to_string();
        meta.sector = sec["sector"].as_str().unwrap_or("").to_string();
        meta.industry = sec["industryInfo"].as_str().unwrap_or("").to_string();
        meta.isin = root["metaData"]["isinCode"].as_str().unwrap_or("").to_string();
        meta.nse_code = root["metaData"]["identifier"].as_str().unwrap_or("").to_string();
        meta.face_value = root["tradeInfo"]["faceValue"].as_f64().map(|f| f.to_string()).unwrap_or_else(|| root["tradeInfo"]["faceValue"].as_i64().map(|i| i.to_string()).unwrap_or_default());
        if let Some(arr) = sec["indexList"].as_array() {
            for idx in arr {
                if let Some(s) = idx.as_str() { meta.indexes.push(s.to_string()); }
            }
        }
    }

    if let Ok(bse) = bse_header {
        if meta.macro_category.is_empty() {
            meta.macro_category = bse["Sector"].as_str().unwrap_or("").to_string();
            meta.sector = bse["IndustryNew"].as_str().unwrap_or("").to_string();
            meta.industry = bse["IGroup"].as_str().unwrap_or("").to_string();
        }
        if meta.isin.is_empty() { meta.isin = bse["ISIN"].as_str().unwrap_or("").to_string(); }
        if meta.face_value.is_empty() { meta.face_value = bse["FaceVal"].as_str().unwrap_or("").to_string(); }
        meta.bse_code = bse["SecurityCode"].as_str().unwrap_or("").to_string();
        if let Some(bse_idx) = bse["Index"].as_str() {
            if !bse_idx.is_empty() && !meta.indexes.iter().any(|x| x == bse_idx) { meta.indexes.push(bse_idx.to_string()); }
        }
    }

    if let Ok(nse_c) = nse_corp {
        if let Some(r10) = nse_c["record10"].as_array().and_then(|a| a.get(0)) {
            meta.nse_listing_date = r10["listingDate"].as_str().unwrap_or("").to_string();
        }
        if let Some(r20) = nse_c["record20"].as_array() {
            for d in r20 {
                let name = d["name"].as_str().unwrap_or("").to_string();
                let des = d["designation"].as_str().unwrap_or("").to_string();
                if !name.is_empty() { meta.directors.push(DirectorRow { name, designation: des }); }
            }
        }
        if let Some(r40) = nse_c["record40"].as_array() {
            let active_addr = r40.iter().find(|r| r["addressType"].as_str() == Some("RG")).or_else(|| r40.get(0));
            if let Some(addr_node) = active_addr {
                let a1 = addr_node["address1"].as_str().unwrap_or("").trim();
                let a2 = addr_node["address2"].as_str().unwrap_or("").trim();
                let a3 = addr_node["address3"].as_str().unwrap_or("").trim();
                let mut full_addr = a1.to_string();
                if !a2.is_empty() { full_addr = format!("{}, {}", full_addr, a2); }
                if !a3.is_empty() { full_addr = format!("{}, {}", full_addr, a3); }
                meta.address = full_addr;
                meta.telephone = addr_node["phoneNo"].as_str().unwrap_or("").to_string();
                meta.fax = addr_node["faxNo"].as_str().unwrap_or("").to_string();
                meta.email = addr_node["emailId"].as_str().unwrap_or("").to_string();
                meta.website = addr_node["website"].as_str().unwrap_or("").to_string();
            }
        }
    }

    if let Ok(bse_d) = bse_directory {
        if let Some(t3) = bse_d["Table3"].as_array().and_then(|a| a.get(0)) {
            let raw_bse_date = t3["lISTING_DATE"].as_str().unwrap_or("");
            meta.bse_listing_date = if raw_bse_date.contains('T') { raw_bse_date.split('T').next().unwrap_or("").to_string() } else { raw_bse_date.to_string() };
        }
        if meta.directors.is_empty() {
            if let Some(table) = bse_d["Table"].as_array() {
                for d in table {
                    let first = d["sFirstname"].as_str().unwrap_or("").trim();
                    let middle = d["sMiddlename"].as_str().unwrap_or("").trim();
                    let last = d["sLastname"].as_str().unwrap_or("").trim();
                    let mut full_name = first.to_string();
                    if !middle.is_empty() { full_name = format!("{} {}", full_name, middle); }
                    if !last.is_empty() { full_name = format!("{} {}", full_name, last); }
                    let designation = d["sDesignation"].as_str().unwrap_or("").to_string();
                    if !full_name.is_empty() && designation.to_lowercase() != "company secretary & compliance officer" {
                        meta.directors.push(DirectorRow { name: full_name, designation });
                    }
                }
            }
        }
        if let Some(t1) = bse_d["Table1"].as_array().and_then(|a| a.get(0)) {
            let bse_tel = t1["Tele"].as_str().unwrap_or("").trim().trim_end_matches(',').to_string();
            let bse_fax = t1["Fax"].as_str().unwrap_or("").trim().trim_end_matches(',').to_string();
            let bse_email = t1["sEmail"].as_str().unwrap_or("").to_string();
            let bse_web = t1["sURL"].as_str().unwrap_or("").to_string();
            if bse_tel.len() > meta.telephone.len() { meta.telephone = bse_tel; }
            if bse_fax.len() > meta.fax.len() { meta.fax = bse_fax; }
            if bse_email.len() > meta.email.len() { meta.email = bse_email; }
            if bse_web.len() > meta.website.len() { meta.website = bse_web; }
        }
    }

    store_parsed_table("overview_metadata", meta);
    Ok(())
}