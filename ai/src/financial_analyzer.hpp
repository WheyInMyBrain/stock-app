#pragma once
#include <string>
#include <filesystem>
#include <vector>
#include <iostream>
#include <map>
#include <unordered_map>
#include <arrow/table.h>
#include <arrow/array.h>
#include <arrow/scalar.h>
#include "data_loader.hpp"
#include "metrics_config.hpp"

namespace fs = std::filesystem;

namespace Processors {

    struct RawDocument {
        std::string raw_date = "";
        std::string nature = "";
        std::string value = "";
    };

    // Helper function to force all date variations (DD-MM-YYYY or YYYY-MM-DD) into strict ISO standard format
    inline std::string normalize_to_iso_date(const std::string& input_date) {
        if (input_date.length() < 10) return input_date;

        // Check if the format is DD-MM-YYYY (has '-' at index 2 and 5)
        if (input_date[2] == '-' && input_date[5] == '-') {
            std::string day   = input_date.substr(0, 2);
            std::string month = input_date.substr(3, 2);
            std::string year  = input_date.substr(6, 4);
            return year + "-" + month + "-" + day;
        }
        
        // Check if the format is DD/MM/YYYY (has '/' at index 2 and 5)
        if (input_date[2] == '/' && input_date[5] == '/') {
            std::string day   = input_date.substr(0, 2);
            std::string month = input_date.substr(3, 2);
            std::string year  = input_date.substr(6, 4);
            return year + "-" + month + "-" + day;
        }

        return input_date; // Already YYYY-MM-DD or unparseable
    }

    inline std::string run_financial_processing(const std::string& data_dir, const std::string& ticker, const MetricConfig& config) {
        
        // Key: Clean ISO Date String (YYYY-MM-DD) -> Value: Raw Metric String Value
        std::map<std::string, std::string> chronological_output;
        
        // Group everything by its absolute source file name context
        std::unordered_map<std::string, RawDocument> file_groups;

        for (const auto& file_name : config.target_files) {
            fs::path target_path = fs::path(data_dir) / ticker / "parquets" / file_name;
            if (!fs::exists(target_path)) continue;

            auto load_result = DataLoader::load_parquet_to_table(target_path.string());
            if (!load_result.ok()) continue;
            std::shared_ptr<arrow::Table> table = load_result.ValueOrDie();

            auto src_file_col = table->GetColumnByName("source_file");
            auto tag_col      = table->GetColumnByName("tag_name");
            auto ctx_col      = table->GetColumnByName("context_id");
            auto val_col      = table->GetColumnByName("raw_value");

            if (!tag_col || !ctx_col || !val_col || !src_file_col) continue;

            for (int64_t row = 0; row < table->num_rows(); ++row) {
                auto src_scalar  = src_file_col->GetScalar(row).ValueOrDie();
                auto tag_scalar  = tag_col->GetScalar(row).ValueOrDie();
                auto ctx_scalar  = ctx_col->GetScalar(row).ValueOrDie();
                auto val_scalar  = val_col->GetScalar(row).ValueOrDie();

                if (!src_scalar->is_valid || !tag_scalar->is_valid || !ctx_scalar->is_valid || !val_scalar->is_valid) {
                    continue;
                }

                std::string file_id = src_scalar->ToString();
                std::string tag     = tag_scalar->ToString();
                std::string ctx     = ctx_scalar->ToString();
                std::string raw_val = val_scalar->ToString();

                std::string tag_lower = tag;
                std::string ctx_lower = ctx;
                std::transform(tag_lower.begin(), tag_lower.end(), tag_lower.begin(), ::tolower);
                std::transform(ctx_lower.begin(), ctx_lower.end(), ctx_lower.begin(), ::tolower);

                auto& doc = file_groups[file_id];

                // 1. Grab raw date text directly and pass it through normalization
                if (tag == "DateOfEndOfReportingPeriod") {
                    doc.raw_date = normalize_to_iso_date(raw_val);
                } 
                // 2. Track nature status text directly
                else if (tag == "NatureOfReportStandaloneConsolidated") {
                    std::string val_lower = raw_val;
                    std::transform(val_lower.begin(), val_lower.end(), val_lower.begin(), ::tolower);
                    doc.nature = val_lower;
                } 
                // 3. Scan matching metric configurations
                else {
                    bool tag_match = false;
                    for (const auto& t : config.xbrl_tags) {
                        if (tag == t) { tag_match = true; break; }
                    }

                    bool ctx_match = false;
                    for (const auto& allowed_ctx : config.contexts) {
                        if (ctx_lower.find(allowed_ctx) != std::string::npos) { ctx_match = true; break; }
                    }

                    if (tag_match && ctx_match) {
                        doc.value = raw_val;
                    }
                }
            }
        }

        // Output matching consolidated documents exactly as they are reported
        for (const auto& [file_id, doc] : file_groups) {
            if (doc.raw_date.empty() || doc.value.empty() || doc.value == "null") continue;
            
            // Enforce Consolidated target rules
            if (doc.nature.find("consolidated") != std::string::npos) {
                
                // If a date collision happens across files, prioritize keeping the larger absolute string value
                if (chronological_output.find(doc.raw_date) == chronological_output.end() || 
                    doc.value.length() > chronological_output[doc.raw_date].length()) {
                    chronological_output[doc.raw_date] = doc.value;
                }
            }
        }

        // Render the clean, standardized results matrix sorted alphabetically by ISO Date
        std::string matrix_output = "date | raw_value\n---|---\n";
        for (const auto& [date, val] : chronological_output) {
            matrix_output += date + " | " + val + "\n";
        }

        if (chronological_output.empty()) {
            return "⚠️ Swept all source files but found no consolidated rows matching tags.";
        }

        return matrix_output;
    }
}