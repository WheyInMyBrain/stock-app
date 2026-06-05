use std::collections::{HashMap, BTreeSet};
use serde_json::{json, Value};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct CashFlowCard;

struct HierarchyRowConfig {
    tag_name: &'static str,
    is_parent: bool,
    parent_id: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 SELF-CONTAINED UTILITY FUNCTIONS
// ─────────────────────────────────────────────────────────────────────────────

fn transform_camel_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        if i == 0 {
            result.push(ch.to_ascii_uppercase());
        } else {
            result.push(ch);
        }
    }
    result
}

fn format_financial_number(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "NA" || trimmed == "NaN" || trimmed == "None" {
        return "-".to_string();
    }
    if let Ok(num) = trimmed.parse::<f64>() {
        if num == 0.0 {
            return "0.00".to_string();
        }
        let crores = num / 10_000_000.0;
        let sign = if crores < 0.0 { "-" } else { "" };
        let abs_crores = crores.abs();
        
        let formatted_str = format!("{:.2}", abs_crores);
        let parts: Vec<&str> = formatted_str.split('.').collect();
        let integer_part = parts[0];
        let decimal_part = parts[1];
        
        let mut grouped_integer = String::new();
        let chars: Vec<char> = integer_part.chars().collect();
        let len = chars.len();
        for (i, ch) in chars.into_iter().enumerate() {
            grouped_integer.push(ch);
            let remaining = len - 1 - i;
            if remaining > 0 && remaining % 3 == 0 {
                grouped_integer.push(',');
            }
        }
        
        if sign == "-" {
            format!("-₹ {}.{} Cr", grouped_integer, decimal_part)
        } else {
            format!("₹ {}.{} Cr", grouped_integer, decimal_part)
        }
    } else {
        trimmed.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 WORKSPACE TRAIT IMPLEMENTATION LAYER
// ─────────────────────────────────────────────────────────────────────────────

impl WorkspaceModule for CashFlowCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "cash_flow_financials".to_string(),
            name: "Cash Flow Statement".to_string(),
            description: "Interactive Ind-As Schedule III Cash Flow timeline mapping operations, investing, and financing items.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 🛠️ DYNAMIC LOADING LAYER: Decode and stream row blocks from the central parquet base
        let parquet_raw_payload = data.get_dataset("parquets/nse_corporates-financial-results.parquet");
        let mut raw_records: Vec<(String, String, String, String)> = Vec::new();

        if let Some(b64_str) = parquet_raw_payload["bytes_base64"].as_str() {
            if let Ok(vec_bytes) = STANDARD.decode(b64_str) {
                let bytes_container = bytes::Bytes::from(vec_bytes);
                if let Ok(file_reader) = SerializedFileReader::new(bytes_container) {
                    let num_groups = file_reader.metadata().num_row_groups();
                    let mut row_group_idx = 0;
                    while row_group_idx < num_groups {
                        if let Ok(group) = file_reader.get_row_group(row_group_idx) {
                            if let Ok(mut row_iter) = group.get_row_iter(None) {
                                while let Some(Ok(row)) = row_iter.next() {
                                    let source_file = row.get_string(0).map(|s| s.to_string()).unwrap_or_default();
                                    let tag_name = row.get_string(1).map(|s| s.to_string()).unwrap_or_default();
                                    let context_id = row.get_string(2).map(|s| s.to_string()).unwrap_or_default();
                                    let raw_value = row.get_string(4).map(|s| s.to_string()).unwrap_or_default();
                                    raw_records.push((source_file, tag_name, context_id, raw_value));
                                }
                            }
                        }
                        row_group_idx += 1;
                    }
                }
            }
        }

        if raw_records.is_empty() {
            return Err("Zero data records parsed from financial registry Parquet".to_string());
        }

        // 🛠️ TIMELINE RECONCILIATION LAYER
        let mut available_types_set = BTreeSet::new();
        let mut file_to_date: HashMap<String, String> = HashMap::new();
        let mut file_to_type: HashMap<String, String> = HashMap::new();
        let mut file_context_to_date: HashMap<(String, String), String> = HashMap::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let val = raw_value.trim().to_string();
            if val.is_empty() || val == "NA" { continue; }

            if tag_name == "DateOfEndOfReportingPeriod" {
                file_to_date.insert(source_file.clone(), val.clone());
                file_context_to_date.insert((source_file.clone(), context_id.clone()), val);
            } else if tag_name == "NatureOfReportStandaloneConsolidated" {
                let report_type_upper = val.to_uppercase();
                file_to_type.insert(source_file.clone(), report_type_upper.clone());
                available_types_set.insert(report_type_upper);
            }
        }

        // 📊 STEP 1: DECODE PERSPECTIVE VIEWS & ACTIVE INTERVAL BOUNDS
        let mut available_types: Vec<String> = available_types_set.into_iter().collect();
        if available_types.is_empty() {
            available_types = vec!["CONSOLIDATED".to_string(), "STANDALONE".to_string()];
        }

        let mut active_report_type = "CONSOLIDATED".to_string();
        let mut active_period_type = "QUARTERLY".to_string();

        let raw_timeframe = timeframe.trim().to_uppercase();
        if !raw_timeframe.is_empty() {
            if raw_timeframe.contains("STANDALONE") {
                active_report_type = "STANDALONE".to_string();
            } else if raw_timeframe.contains("CONSOLIDATED") {
                active_report_type = "CONSOLIDATED".to_string();
            }

            if raw_timeframe.contains("ANNUAL") || raw_timeframe.contains("YEAR") {
                active_period_type = "ANNUALLY".to_string();
            } else if raw_timeframe.contains("QUARTER") {
                active_period_type = "QUARTERLY".to_string();
            }
        }

        if !available_types.contains(&active_report_type) {
            active_report_type = available_types[0].clone();
        }

        // 🎯 DURATION ROUTING CONTEXT: Directs to period thresholds (OneD for Quarterly, FourD for Annual)
        let target_context_id = if active_period_type == "ANNUALLY" { "FourD" } else { "OneD" };

        let current_active_select_label = format!(
            "{} - {}", 
            transform_camel_case(&active_report_type.to_lowercase()), 
            if active_period_type == "ANNUALLY" { "Annually" } else { "Quarterly" }
        );

        let mut unified_dropdown_options = Vec::new();
        for t in &available_types {
            let t_label = transform_camel_case(&t.to_lowercase());
            unified_dropdown_options.push(format!("{} - Quarterly", t_label));
            unified_dropdown_options.push(format!("{} - Annually", t_label));
        }

        // 📊 STEP 2: CONSTRUCT MATRIX GRID TIME COORDINATES
        // 📊 STEP 2: CONSTRUCT MATRIX GRID TIME COORDINATES
        let mut unique_filing_dates: Vec<String> = Vec::new();
        let mut matrix_data_map: HashMap<String, String> = HashMap::new();
        
        // 🎯 COUNTER LOCK: Tracks tag sequence iterations per filing date to resolve duplicate cash tags
        let mut occurrence_counter: HashMap<String, u32> = HashMap::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let true_date = file_context_to_date.get(&(source_file.clone(), context_id.clone()))
                .cloned()
                .unwrap_or_else(|| file_to_date.get(source_file).cloned().unwrap_or_default());
                
            let report_type = file_to_type.get(source_file).cloned().unwrap_or_default();

            if true_date.is_empty() { continue; }

            if report_type.contains(&active_report_type) && context_id == target_context_id {
                // MARCH ENDPOINT THRESHOLD FOR FULL AUDITED ANNUAL REPORTS
                if target_context_id == "FourD" && !true_date.contains("-03-") {
                    continue;
                }

                if !unique_filing_dates.contains(&true_date) {
                    unique_filing_dates.push(true_date.clone());
                }
                
                // 🎯 COLLISION RESOLUTION LAYER
                let mut final_tag = tag_name.clone();
                if tag_name == "CashAndCashEquivalentsCashFlowStatement" {
                    let counter_key = format!("{}__{}", true_date, tag_name);
                    let current_count = occurrence_counter.entry(counter_key).or_insert(0);
                    *current_count += 1;
                    
                    if *current_count == 1 {
                        final_tag = "CashAndCashEquivalentsCashFlowStatementBeginning".to_string();
                    } else {
                        final_tag = "CashAndCashEquivalentsCashFlowStatementEnding".to_string();
                    }
                }

                let data_lookup_key = format!("{}__{}", true_date, final_tag);
                // Use .insert() instead of .or_insert_with() to ensure clean, precise tracking capture
                matrix_data_map.insert(data_lookup_key, raw_value.clone());
            }
        }

        // Sort descending to place newest dates into the leftmost matrix columns
        unique_filing_dates.sort_by(|a, b| b.cmp(a));

        // 📊 STEP 3: IND-AS SCHEDULE III CASH FLOW SEQUENTIAL CONFIG TREE
        let structured_cashflow_tree = vec![
            // ─── OPERATING ACTIVITIES ACCORDION CLUSTER ───────────────────────────────
            HierarchyRowConfig { tag_name: "CashFlowsFromUsedInOperatingActivities", is_parent: true, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "ProfitBeforeTax", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForFinanceCosts", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDecreaseIncreaseInInventories", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDecreaseIncreaseInTradeReceivablesCurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDecreaseIncreaseInTradeReceivablesNoncurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDecreaseIncreaseInOtherCurrentAssets", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDecreaseIncreaseInOtherNoncurrentAssets", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForOtherFinancialAssetsNoncurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForOtherFinancialAssetsCurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForOtherBankBalances", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForIncreaseDecreaseInTradePayablesCurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForIncreaseDecreaseInTradePayablesNoncurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForIncreaseDecreaseInOtherCurrentLiabilities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForIncreaseDecreaseInOtherNoncurrentLiabilities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDepreciationAndAmortisationExpense", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForImpairmentLossReversalOfImpairmentLossRecognisedInProfitOrLoss", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForProvisionsCurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForProvisionsNoncurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForOtherFinancialLiabilitiesCurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForOtherFinancialLiabilitiesNoncurrent", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForUnrealisedForeignExchangeLossesGains", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForDividendIncome", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForInterestIncome", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForSharebasedPayments", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForFairValueGainsLosses", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForUndistributedProfitsOfAssociates", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "OtherAdjustmentsForWhichCashEffectsAreInvestingOrFinancingCashFlow", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "OtherAdjustmentsToReconcileProfitLoss", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "OtherAdjustmentsForNoncashItems", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "ShareOfProfitAndLossFromPartnershipFirmOrAssociationOfPersonsOrLimitedLiabilityPartnerships", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "AdjustmentsForReconcileProfitLoss", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "CashFlowsFromUsedInOperations", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "DividendsReceivedClassifiedAsOperatingActivities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "InterestPaidClassifiedAsOperatingActivities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "InterestReceivedClassifiedAsOperatingActivities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "IncomeTaxesPaidRefundClassifiedAsOperatingActivities", is_parent: false, parent_id: "operating_activities_group" },
            HierarchyRowConfig { tag_name: "OtherInflowsOutflowsOfCashClassifiedAsOperatingActivities", is_parent: false, parent_id: "operating_activities_group" },

            // ─── INVESTING ACTIVITIES ACCORDION CLUSTER ───────────────────────────────
            HierarchyRowConfig { tag_name: "CashFlowsFromUsedInInvestingActivities", is_parent: true, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashFlowsFromLosingControlOfSubsidiariesOrOtherBusinessesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashFlowsUsedInObtainingControlOfSubsidiariesOrOtherBusinessesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherCashReceiptsFromSalesOfEquityOrDebtInstrumentsOfOtherEntitiesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherCashPaymentsToAcquireEquityOrDebtInstrumentsOfOtherEntitiesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherCashReceiptsFromSalesOfInterestsInJointVenturesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherCashPaymentsToAcquireInterestsInJointVenturesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashReceiptsFromShareOfProfitsOfPartnershipFirmOrAssociationOfPersonsOrLimitedLiabilityPartnerships", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashPaymentForInvestmentInPartnershipFirmOrAssociationOfPersonsOrLimitedLiabilityPartnerships", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfInvestmentPropertyClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfInvestmentPropertyClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfIntangibleAssetsUnderDevelopment", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfIntangibleAssetsUnderDevelopment", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfGoodwillClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfGoodwillClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfIntangibleAssetsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfIntangibleAssetsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromBiologicalAssetsOtherThanBearerPlantsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfBiologicalAssetsOtherThanBearerPlantsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromSalesOfOtherLongTermAssetsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "PurchaseOfOtherLongTermAssetsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashAdvancesAndLoansMadeToOtherPartiesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashReceiptsFromRepaymentOfAdvancesAndLoansMadeToOtherPartiesClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashPaymentsForFutureContractsForwardContractsOptionContractsAndSwapContractsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "CashReceiptsFromFutureContractsForwardContractsOptionContractsAndSwapContractsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "DividendsReceivedClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "InterestReceivedClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "IncomeTaxesPaidRefundClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherInflowsOutflowsOfCashClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromGovernmentGrantsClassifiedAsInvestingActivities", is_parent: false, parent_id: "investing_activities_group" },

            // ─── FINANCING ACTIVITIES ACCORDION CLUSTER ───────────────────────────────
            HierarchyRowConfig { tag_name: "CashFlowsFromUsedInFinancingActivities", is_parent: true, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromChangesInOwnershipInterestsInSubsidiaries", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "PaymentsFromChangesInOwnershipInterestsInSubsidiaries", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromIssuingSharesClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromIssuingOtherEquityInstruments", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "PaymentsToAcquireOrRedeemEntitysShares", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "PaymentsOfOtherEquityInstruments", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromExerciseOfStockOptions", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromIssuingDebenturesNotesBondsEtc", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "ProceedsFromBorrowingsClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "RepaymentsOfBorrowingsClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "PaymentsOfFinanceLeaseLiabilitiesClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "PaymentsOfLeaseLiabilitiesClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "DividendsPaidClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "InterestPaidClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "IncomeTaxesPaidRefundClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },
            HierarchyRowConfig { tag_name: "OtherInflowsOutflowsOfCashClassifiedAsFinancingActivities", is_parent: false, parent_id: "financing_activities_group" },

            // ─── RECONCILIATION OVERHEAD SUMMARY TOTALS (ROOT ANCHORS) ────────────────────
            HierarchyRowConfig { tag_name: "IncreaseDecreaseInCashAndCashEquivalentsBeforeEffectOfExchangeRateChanges", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "EffectOfExchangeRateChangesOnCashAndCashEquivalents", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "IncreaseDecreaseInCashAndCashEquivalents", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "CashAndCashEquivalentsCashFlowStatementBeginning", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "CashAndCashEquivalentsCashFlowStatementEnding", is_parent: false, parent_id: "" },
        ];

        // 📊 STEP 4: DATA MATRIX GRID ROW COMPILER
        let mut compiled_rows = Vec::new();
        let mut table_headers = vec!["Schedule III Cash Flow Component".to_string()];
        
        for date in &unique_filing_dates {
            table_headers.push(date.clone());
        }

        for config in &structured_cashflow_tree {
            let mut row_cells = Vec::new();
            let clean_row_header = match config.tag_name {
                "CashAndCashEquivalentsCashFlowStatementBeginning" => "Cash And Cash Equivalents At Beginning Of Period".to_string(),
                "CashAndCashEquivalentsCashFlowStatementEnding" => "Cash And Cash Equivalents At End Of Period".to_string(),
                _ => transform_camel_case(config.tag_name),
            };

            row_cells.push(json!({ "type": "text", "value": clean_row_header }));

            for date in &unique_filing_dates {
                let lookup_key = format!("{}__{}", date, config.tag_name);
                let raw_amount = matrix_data_map.get(&lookup_key).map(|s| s.as_str()).unwrap_or("");
                let formatted_amount = format_financial_number(raw_amount);

                row_cells.push(json!({ "type": "text", "value": formatted_amount }));
            }

            let has_parent_group = !config.parent_id.is_empty();
            let is_child_row = has_parent_group && !config.is_parent;

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": config.is_parent,
                "is_child": is_child_row,
                "parent_id": if has_parent_group { Some(config.parent_id.to_string()) } else { None },
                "align_right_values": true,
                "cells": row_cells
            }));
        }

        // 📊 STEP 5: OUTPUT ACCORDION-SUPPORTED UI SCHEMATICS
        Ok(json!({
            "type": "card",
            "title": format!("{} Cash Flow Statement ({})", transform_camel_case(&active_report_type.to_lowercase()), if active_period_type == "ANNUALLY" { "Annually" } else { "Quarterly" }),
            "subtitle": format!("// PERIOD RECONCILIATION FILTER // {}", current_active_select_label),
            "footer": format!("Total active cash flow lines: {} statement metrics across {} intervals", structured_cashflow_tree.len(), unique_filing_dates.len()),
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-row justify-between items-center w-full mt-1 mb-4 pointer-events-auto",
                    "style": { "display": "flex", "flexDirection": "row", "justifyContent": "between" },
                    "children": [
                        {
                            "type": "text",
                            "className": "text-xs font-semibold font-mono uppercase opacity-60 text-neutral-400", 
                            "value": "Cash Flow Interval Selector:"
                        },
                        {
                            "type": "select",
                            "action_target": "cash_flow_financials", 
                            "default_value": current_active_select_label,
                            "options": unified_dropdown_options
                        }
                    ]
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": compiled_rows
                        }
                    ]
                }
            ]
        }))
    }
}