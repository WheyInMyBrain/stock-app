// stock-app/ui/backend/src/database/overview.rs

use std::collections::{BTreeSet, HashMap};
use polars::prelude::*;
use crate::commands::memory_pool::store_parsed_table;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DirectorRow {
    pub name: String,
    pub designation: String,
}

// =========================================================================
// NEW STRATEGIC SHAREHOLDING STRUCTS
// =========================================================================

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HistoricalMacroAllocationRow {
    pub date: String,
    pub category_label: String,
    pub share_count: f64,
    pub stake_percentage: f64,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HistoricalWhaleRow {
    pub date: String,
    pub investor_name: String,
    pub entity_classification: String,
    pub share_count: f64,
    pub stake_percentage: f64,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HistoricalSboRow {
    pub date: String,
    pub human_sbo_name: String,
    pub proxy_registered_owner: String,
    pub nationality: String,
    pub acquisition_date: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoricalComplaintRow {
    pub date: String,
    pub complaints_beginning: f64,
    pub complaints_received: f64,
    pub complaints_disposed: f64,
    pub complaints_unresolved: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerRecord {
    pub symbol: String,
    pub ltp: f64,
    pub p_change: f64,
    pub market_cap: f64,
    pub pe: f64,
    pub eps: f64,
    pub pat: f64,
    pub total_income: f64,
    pub promoter_holding: f64,
    pub debt_eq_ratio: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerCategory {
    pub category_name: String,
    pub available_dates: Vec<String>,
    pub date_matrices: std::collections::HashMap<String, Vec<PeerRecord>>,
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
    pub available_reporting_dates: Vec<String>,
    pub macro_allocations: Vec<HistoricalMacroAllocationRow>,
    pub hni_whales: Vec<HistoricalWhaleRow>,
    pub sbo_registry: Vec<HistoricalSboRow>,
    pub investor_complaints: Vec<HistoricalComplaintRow>,
    pub peer_comparisons: Vec<PeerCategory>,
}

// =========================================================================
// COMPACTION AND NORMALIZATION HELPERS
// =========================================================================

fn classify_entity_type(name: &str, categories: &str) -> String {
    let combined = format!("{} {}", name.to_uppercase(), categories.to_uppercase());
    if combined.contains("TRUST") {
        "Family Trust / Employee Benefit".to_string()
    } else if combined.contains("LTD") || combined.contains("PVT") || combined.contains("LIMITED") || combined.contains("BODIES CORPORATE") {
        "Corporate Body / Entity".to_string()
    } else if combined.contains("CLEARING") {
        "Clearing Member".to_string()
    } else {
        "Individual / HNI / Promoter".to_string()
    }
}

// =========================================================================
// MAIN ORCHESTRATION PIPELINE
// =========================================================================

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
    
    // =========================================================================
    // DUAL-EXCHANGE SHAREHOLDING EXTRACTION CORE (FIXED POSITION INDEX MATCHING)
    // =========================================================================
    let nse_bytes_opt = loader.load_raw_bytes("parquets/nse_corporate-shareholding-master.parquet").ok();
    let bse_bytes_opt = loader.load_raw_bytes("parquets/bse_shareholding-pattern-docs.parquet").ok();

    let mut chosen_reporting_dates = Vec::new();
    let mut chosen_macro_allocations = Vec::new();
    let mut chosen_hni_whales = Vec::new();
    let mut chosen_sbo_registry = Vec::new();
    let mut highest_record_count = 0;

    let source_variants = vec![
        ("NSE-Master", nse_bytes_opt),
        ("BSE-Docs", bse_bytes_opt)
    ];

    for (_source_tag, bytes_payload) in source_variants {
        let raw_bytes = match bytes_payload {
            Some(b) => b,
            None => continue,
        };

        if let Ok(df) = ParquetReader::new(std::io::Cursor::new(raw_bytes)).finish() {
            let mut file_to_report_date = HashMap::new();
            let mut unique_report_dates = BTreeSet::new();
            let mut master_coordinate_matrix = HashMap::new();
            let mut dynamic_whale_keys = BTreeSet::new();
            let mut dynamic_sbo_indices = BTreeSet::new();

            if df.width() >= 5 {
                // Fix: Extract from the option reference using standard combinators safely
                let f_ca = df.select_at_idx(0).and_then(|c| c.str().ok());
                let p_ca = df.select_at_idx(1).and_then(|c| c.str().ok()); 
                let ctx_ca = df.select_at_idx(2).and_then(|c| c.str().ok()); 
                let curr_ca = df.select_at_idx(4).and_then(|c| c.str().ok()); 

                if let (Some(f_ca), Some(p_ca), Some(ctx_ca), Some(curr_ca)) = (f_ca, p_ca, ctx_ca, curr_ca) {
                    
                    // PASS 1: Identify explicit reporting quarter targets grouped by file keys
                    for i in 0..df.height() {
                        let source_file = f_ca.get(i).unwrap_or_default().to_string();
                        let tag_name = p_ca.get(i).unwrap_or_default();
                        let raw_value = curr_ca.get(i).unwrap_or_default().trim().to_string();

                        if tag_name == "DateOfReport" && !raw_value.is_empty() && raw_value != "NA" && raw_value != "0" {
                            file_to_report_date.insert(source_file, raw_value);
                        }
                    }

                    // PASS 2: Group rows matching file codes to prevent context collisions
                    for i in 0..df.height() {
                        let source_file = f_ca.get(i).unwrap_or_default().to_string();
                        let tag_name = p_ca.get(i).unwrap_or_default().to_string();
                        let context_id = ctx_ca.get(i).unwrap_or_default().to_string();
                        let raw_value = curr_ca.get(i).unwrap_or_default().trim().to_string();

                        if context_id.to_lowercase() != "consolidated" && !context_id.is_empty() {
                            if let Some(true_date) = file_to_report_date.get(&source_file) {
                                if tag_name == "NumberOfShares" && !raw_value.is_empty() && raw_value != "NA" && raw_value != "0" {
                                    unique_report_dates.insert(true_date.clone());
                                }

                                let lookup_key = format!("{}__{}__{}", true_date, context_id, tag_name);
                                master_coordinate_matrix.insert(lookup_key, raw_value.clone());

                                let len = context_id.len();
                                if len >= 4 {
                                    let suffix = &context_id[len - 4..];
                                    let chars: Vec<char> = suffix.chars().collect();
                                    if chars.len() == 4 && chars[0].is_ascii_digit() && chars[1].is_ascii_digit() && chars[2].is_ascii_digit() && (chars[3] == 'D' || chars[3] == 'I') {
                                        let prefix_base = context_id[..len - 4].to_string();
                                        let index_code = format!("{}{}{}", chars[0], chars[1], chars[2]);

                                        let ctx_lower = context_id.to_lowercase();
                                        let tag_lower = tag_name.to_lowercase();

                                        if ctx_lower.contains("significant") || ctx_lower.contains("sbo") || tag_lower.contains("beneficial") {
                                            dynamic_sbo_indices.insert(index_code);
                                        } else {
                                            dynamic_whale_keys.insert((prefix_base, index_code));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let chronological_dates: Vec<String> = unique_report_dates.into_iter().rev().collect();
                    let matrix_lookup = |date: &str, ctx: &str, tag: &str| -> String {
                        let key = format!("{}__{}__{}", date, ctx, tag);
                        master_coordinate_matrix.get(&key).cloned().unwrap_or_default()
                    };

                    let matrix_lookup_f64 = |date: &str, ctx: &str, tag: &str| -> f64 {
                        matrix_lookup(date, ctx, tag).parse::<f64>().unwrap_or(0.0)
                    };

                    let mut variant_macro_allocations = Vec::new();
                    let mut variant_hni_whales = Vec::new();
                    let mut variant_sbo_registry = Vec::new();

                    // Pipeline Step 1: Parse structural macro allocation tree variables
                    let tracked_roots = vec![
                        ("ShareholdingOfPromoterAndPromoterGroupI", "Promoters & Promoter Group"),
                        ("InstitutionsI", "Institutional Open Market Float"),
                        ("MutualFundsOrUtiI", "↳ Institutional Mutual Funds"),
                        ("InstitutionsForeignPortfolioInvestorI", "↳ Foreign Portfolio Investors (FPI)"),
                        ("NonInstitutionsI", "Public Non-Institutional Float"),
                        ("IndividualShareholdersHoldingNominalShareCapitalUpToRs2LakhsI", "↳ Retail Public (Nominal Up to ₹2 Lk)"),
                        ("IndividualShareholdersHoldingNominalShareCapitalInExcessOfRs2LakhsI", "↳ HNI Public Whales (Above ₹2 Lk)"),
                    ];

                    for date in &chronological_dates {
                        let total_pattern_shares = matrix_lookup_f64(date, "ShareholdingPatternI", "NumberOfShares");
                        for (ctx_id, visual_label) in &tracked_roots {
                            let shares = matrix_lookup_f64(date, ctx_id, "NumberOfShares");
                            if shares > 0.0 {
                                let percentage = if total_pattern_shares > 0.0 { (shares / total_pattern_shares) * 100.0 } else { 0.0 };
                                variant_macro_allocations.push(HistoricalMacroAllocationRow {
                                    date: date.to_string(), // Fixed: Use true string allocation mapping methods
                                    category_label: visual_label.to_string(),
                                    share_count: shares,
                                    stake_percentage: percentage,
                                });
                            }
                        }
                    }

                    // Pipeline Step 2: Unroll exact corporate insider/HNI whale blocks
                    for (prefix_base, index_str) in &dynamic_whale_keys {
                        let context_coordinate_d = format!("{}{}{}", prefix_base, index_str, "D");
                        let context_coordinate_i = format!("{}{}{}", prefix_base, index_str, "I");

                        for date in &chronological_dates {
                            let name = matrix_lookup(date, &context_coordinate_d, "NameOfTheShareholder");
                            let shares = matrix_lookup_f64(date, &context_coordinate_i, "NumberOfShares");
                            let total_cap = matrix_lookup_f64(date, "ShareholdingPatternI", "NumberOfShares");

                            if !name.is_empty() && name != "NA" && name != "0" && shares > 0.0 {
                                let cat_probe = matrix_lookup(date, &context_coordinate_d, "CategoryOfOtherIndianShareholders");
                                let class_tag = classify_entity_type(&name, &cat_probe);
                                let weight = if total_cap > 0.0 { (shares / total_cap) * 100.0 } else { 0.0 };

                                variant_hni_whales.push(HistoricalWhaleRow {
                                    date: date.to_string(), // Fixed: Use true string allocation mapping methods
                                    investor_name: name,
                                    entity_classification: class_tag,
                                    share_count: shares,
                                    stake_percentage: weight,
                                });
                            }
                        }
                    }

                    // Pipeline Step 3: Unroll absolute human Significant Beneficial Ownership (SBO) records
                    for index_str in &dynamic_sbo_indices {
                        let context_variants = vec![
                            format!("Significant Beneficial Owners Axis{}D", index_str),
                            format!("Significant Beneficial Owners Axis{}I", index_str),
                            format!("SignificantBeneficialOwnersAxis{}D", index_str),
                            format!("SignificantBeneficialOwnersAxis{}I", index_str),
                            format!("SignificantBeneficialOwners{}D", index_str),
                            format!("SignificantBeneficialOwners{}I", index_str),
                            format!("SignificantBeneficialOwner{}D", index_str),
                            format!("SignificantBeneficialOwner{}I", index_str),
                        ];

                        for date in &chronological_dates {
                            for ctx in &context_variants {
                                let sbo_name = matrix_lookup(date, ctx, "NameOfSignificantBeneficialOwners");
                                let proxy_name = matrix_lookup(date, ctx, "NameOfRegisteredOwner");
                                let nat = matrix_lookup(date, ctx, "NationalityOfSignificantBeneficialOwners");
                                let acq = matrix_lookup(date, ctx, "DateOfCreationOrAcquisitionOfSignificantBeneficialInterest");

                                if !sbo_name.is_empty() && sbo_name != "NA" && sbo_name != "-" {
                                    variant_sbo_registry.push(HistoricalSboRow {
                                        date: date.to_string(),
                                        human_sbo_name: sbo_name,
                                        proxy_registered_owner: if proxy_name.is_empty() || proxy_name == "NA" { "Direct Shareholding Body".to_string() } else { proxy_name },
                                        nationality: if nat.is_empty() || nat == "NA" { "Indian".to_string() } else { nat },
                                        acquisition_date: if acq.is_empty() || acq == "NA" { "Nominal Setup Base".to_string() } else { acq },
                                    });
                                    break; // Match confirmed; break context loop variant pass safely
                                }
                            }
                        }
                    }

                    if !chronological_dates.is_empty() {
                        let total_variant_records = variant_macro_allocations.len() + variant_hni_whales.len();
                        if total_variant_records >= highest_record_count {
                            highest_record_count = total_variant_records;
                            chosen_reporting_dates = chronological_dates;
                            chosen_macro_allocations = variant_macro_allocations;
                            chosen_hni_whales = variant_hni_whales;
                            chosen_sbo_registry = variant_sbo_registry;
                        }
                    }
                }
            }
        }
    }

    meta.available_reporting_dates = chosen_reporting_dates;
    meta.macro_allocations = chosen_macro_allocations;
    meta.hni_whales = chosen_hni_whales;
    meta.sbo_registry = chosen_sbo_registry;

    let nse_json = loader
        .load_json_struct::<serde_json::Value>("nse_investor-complaints/endpoint-metadata.json")
        .unwrap_or_default();

    let mut complaint_timeline = Vec::new();
    
    let filings_pool = if let Some(arr) = nse_json.as_array() {
        arr.clone()
    } else if let Some(arr) = nse_json.get("data").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };

    let parse_to_sortable_key = |date_str: &str| -> String {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() == 3 {
            let day = parts[0];
            let month_str = parts[1].to_lowercase();
            let year = parts[2];

            let month_num = match month_str.as_str() {
                "jan" => "01", "feb" => "02", "mar" => "03", "apr" => "04",
                "may" => "05", "jun" => "06", "jul" => "07", "aug" => "08",
                "sep" => "09", "oct" => "10", "nov" => "11", "dec" => "12",
                _ => "00",
            };
            format!("{}{}{}", year, month_num, day)
        } else {
            "00000000".to_string() 
        }
    };

    for filing in &filings_pool {
        let date_str = filing.get("date").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
        
        let get_f64_field = |key: &str| -> f64 {
            if let Some(val) = filing.get(key) {
                if let Some(s) = val.as_str() {
                    s.parse::<f64>().unwrap_or(0.0)
                } else {
                    val.as_f64().unwrap_or(0.0)
                }
            } else {
                0.0
            }
        };

        let beg = get_f64_field("complBeg");
        let recv = get_f64_field("complRecv");
        let disp = get_f64_field("complDisp");
        let unres = get_f64_field("complUnres");

        complaint_timeline.push(HistoricalComplaintRow {
            date: date_str,
            complaints_beginning: beg,
            complaints_received: recv,
            complaints_disposed: disp,
            complaints_unresolved: unres,
        });
    }

    complaint_timeline.sort_by(|a, b| {
        let key_a = parse_to_sortable_key(&a.date);
        let key_b = parse_to_sortable_key(&b.date);
        key_b.cmp(&key_a) 
    });

    meta.investor_complaints = complaint_timeline;

    // =========================================================================
    // SYSTEM PEER COMPARISON CORE (ZERO HARDCODED SYSTEM PATHS)
    // =========================================================================
    let mut category_map: std::collections::HashMap<String, std::collections::HashMap<String, Vec<PeerRecord>>> = std::collections::HashMap::new();
    
    // Pass the logical route straight into your internal path resolver engine
    let target_matrix_dir = "nse_peer-comparison-matrix";

    if let Ok(file_names) = loader.load_directory_filenames(target_matrix_dir) {
        for name_token in file_names {
            if name_token.ends_with(".json") {
                // Strip the .json extension to parse categories and dates cleanly
                let file_stem = name_token.trim_end_matches(".json");
                let mut parts: Vec<&str> = file_stem.split('_').collect();
                
                if parts.len() >= 2 {
                    let date_str = parts.pop().unwrap().to_string();
                    let category_str = parts.join(" ");

                    // Use your asset framework to fetch file data via its logical location string
                    let target_logical_file_path = format!("{}/{}", target_matrix_dir, name_token);
                    if let Ok(json_val) = loader.load_json_struct::<serde_json::Value>(&target_logical_file_path) {
                        let mut records = Vec::new();
                        let data_arr = json_val.as_array().or_else(|| json_val.get("data").and_then(|d| d.as_array()));

                        if let Some(arr) = data_arr {
                            for item in arr {
                                let get_f64 = |k: &str| -> f64 {
                                    item.get(k).and_then(|v| {
                                        if let Some(n) = v.as_f64() { Some(n) }
                                        else if let Some(s) = v.as_str() { s.parse::<f64>().ok() }
                                        else { None }
                                    }).unwrap_or(0.0)
                                };

                                let symbol = item.get("symbol").and_then(|v| v.as_str()).unwrap_or("N/A").to_string();
                                let debt_eq = match item.get("debtEqRatio") {
                                    Some(v) if v.is_string() => v.as_str().unwrap().to_string(),
                                    Some(v) if v.is_number() => v.to_string(),
                                    _ => "N/A".to_string()
                                };

                                records.push(PeerRecord {
                                    symbol,
                                    ltp: get_f64("ltp"),
                                    p_change: get_f64("pChange").max(get_f64("PChange")),
                                    market_cap: get_f64("marketCap"),
                                    pe: get_f64("pe"),
                                    eps: get_f64("eps"),
                                    pat: get_f64("pat"),
                                    total_income: get_f64("totalIncome"),
                                    promoter_holding: get_f64("promoterHolding"),
                                    debt_eq_ratio: debt_eq,
                                });
                            }
                        }
                        category_map.entry(category_str).or_default().insert(date_str, records);
                    }
                }
            }
        }
    }

    let mut peer_comparisons = Vec::new();
    for (cat_name, date_map) in category_map {
        let mut dates: Vec<String> = date_map.keys().cloned().collect();
        dates.sort_by(|a, b| b.cmp(a)); 

        peer_comparisons.push(PeerCategory {
            category_name: cat_name,
            available_dates: dates,
            date_matrices: date_map,
        });
    }
    
    peer_comparisons.sort_by(|a, b| a.category_name.cmp(&b.category_name));
    meta.peer_comparisons = peer_comparisons;

    store_parsed_table("overview_metadata", meta);
    Ok(())
}