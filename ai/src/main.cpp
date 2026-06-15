#include <iostream>
#include <string>
#include <filesystem>
#include "ai_engine.hpp"
#include "ai_router.hpp"
#include "ai_analyst.hpp" 
#include "data_loader.hpp"
#include "metrics_config.hpp"
#include "financial_analyzer.hpp"

namespace fs = std::filesystem;

int main(int argc, char* argv[]) {
    std::string user_query = "";
    std::string data_dir = "";
    std::string ticker = "";

    for (int i = 1; i < argc; ++i) {
        if (std::string(argv[i]) == "--query" && i + 1 < argc) user_query = argv[++i];
        if (std::string(argv[i]) == "--data-dir" && i + 1 < argc) data_dir = argv[++i];
        if (std::string(argv[i]) == "--ticker" && i + 1 < argc) ticker = argv[++i];
    }

    if (user_query.empty() || data_dir.empty() || ticker.empty()) {
        std::cerr << "❌ Usage: ./ai_agent --data-dir <PATH> --ticker <TICKER> --query <QUESTION>" << std::endl;
        return 1;
    }

    fs::path base_data_path = fs::absolute(data_dir);
    fs::path model_path = base_data_path / "Qwen3.5-4B-UD-Q4_K_XL.gguf";

    // 1. Initialize the Core AI Hardware Backend Contexts
    AIEngine engine;
    engine.verify_and_download_model(model_path.string());
    if (!engine.initialize(model_path.string())) return 1;

    // 2. Step 1: Run Intent Extraction
    AIRouter router;
    std::string outcome_tag = router.resolve_intent_tag(engine, user_query);

    // 3. Look up instructions mapping to the extracted tag
    auto it = MetricRegistry.find(outcome_tag);
    if (it == MetricRegistry.end()) {
        std::cerr << "❌ Configuration mapping error: Tag not found in registry." << std::endl;
        return 1;
    }

    // 4. Run Data Extraction Engine Natively
    std::string matrix_output = it->second.run_function(data_dir, ticker, it->second);

    // Print Data Matrix Output Boundary
    std::cout << "__DATA_OUTPUT_START__\n" << matrix_output << "\n__DATA_OUTPUT_END__\n" << std::endl;

    // 5. 🚀 STEP 3: Run AI Explanation & Analysis (Boundaries handled seamlessly inside header!)
    AIAnalyst analyst;
    analyst.generate_explanation(engine, user_query, ticker, matrix_output);

    return 0;
}