#pragma once
#include <string>
#include <vector>
#include <unordered_map>

struct MetricConfig;

using ProcessorFunc = std::string (*)(const std::string& data_dir, const std::string& ticker, const MetricConfig& config);

struct MetricConfig {
    std::string ai_description;         
    std::vector<std::string> target_files;
    std::vector<std::string> xbrl_tags;  // 🚀 SYNONYM ARRAY: Maps all possible alternate tags cleanly
    std::vector<std::string> contexts;  
    ProcessorFunc run_function;         
};

namespace Processors {
    std::string run_financial_processing(const std::string& data_dir, const std::string& ticker, const MetricConfig& config);
}

const std::unordered_map<std::string, MetricConfig> MetricRegistry = {
    {
        "PROFIT", {
            "Net income, profit after tax, or bottom-line corporate earnings",
            {
                "bse_financial-results-docs.parquet",
                "bse_integrated-finance-data.parquet",
                "nse_integrated-finance-results.parquet",
                "nse_corporates-financial-results.parquet"
            }, 
            {
                "ProfitLossForPeriod", 
                "ComprehensiveIncomeForThePeriodAttributableToOwnersOfParent", 
                "ProfitOrLossAttributableToOwnersOfParent"
            },
            {"oned"},
            Processors::run_financial_processing 
        }
    },
    {
        "EPS", {
            "Earnings per share, diluted EPS, or allocations per outstanding share",
            {
                "bse_financial-results-docs.parquet",
                "bse_integrated-finance-data.parquet",
                "nse_integrated-finance-results.parquet",
                "nse_corporates-financial-results.parquet"
            },
            {
                "BasicEarningsLossPerShareFromContinuingAndDiscontinuedOperations",
                "BasicEarningsLossPerShareFromContinuingOperations"
            },
            {"oned"},
            Processors::run_financial_processing 
        }
    }
};

inline std::string build_gbnf_grammar_from_registry() {
    std::string gbnf = "root ::= ";
    size_t i = 0;
    for (const auto& [tag, _] : MetricRegistry) {
        gbnf += "\"" + tag + "\"";
        if (i < MetricRegistry.size() - 1) gbnf += " | ";
        i++;
    }
    return gbnf;
}

inline std::string build_schema_description_from_registry() {
    std::string context = "Available Metric Schema Options:\n";
    for (const auto& [tag, meta] : MetricRegistry) {
        context += "- Tag: [" + tag + "] -> Focuses on: " + meta.ai_description + "\n";
    }
    return context;
}