#include <iostream>
#include <string>
#include <vector>
#include <filesystem>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <thread>
#include <algorithm>
#include "llama.h"
#include "common.h"
#include "sampling.h"

namespace fs = std::filesystem;

// Background downloader targeting the absolute local folder path
void download_model_via_curl(const std::string& target_path) {
    if (fs::exists(target_path)) return;

    std::cout << "🤖 First launch initializing. Downloading Qwen 3.5 Core (2.99 GB)..." << std::endl;
    std::cout << "📍 Destination: " << target_path << std::endl;
    
    std::string url = "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-UD-Q4_K_XL.gguf";
    std::string cmd = "curl -L -o \"" + target_path + "\" " + url;
    
    int result = std::system(cmd.c_str());
    if (result != 0) {
        std::cerr << "❌ System Error: Execution layer failed to stream file via curl command." << std::endl;
        std::exit(1);
    }
    std::cout << "✅ Model checkpoint successfully downloaded and cached inside the project!" << std::endl;
}

// 🎯 LOCATION-AWARE FILE LOADER: Maps to the shared central storage folder safely
std::string load_corporate_financial_context(const fs::path& ai_root_dir, const std::string& ticker) {
    // Project Root Resolution: backs out of 'ai/' to find 'stock-app/' root directory level
    fs::path project_root = ai_root_dir.parent_path(); 
    
    // Construct absolute target file path
    fs::path target_path = project_root / "data" / ticker / "parquets" / "annual_report" / "income_statement.txt";
    
    std::cout << "📂 Searching for target summary file at: " << fs::absolute(target_path) << std::endl;

    if (!fs::exists(target_path)) {
        return "Warning: Clear textual statement summary not found for ticker " + ticker + ". Processing query over general constraints.";
    }

    std::ifstream file(target_path);
    std::stringstream buffer;
    buffer << file.rdbuf();
    return buffer.str();
}

int main(int argc, char* argv[]) {
    if (argc < 5) {
        std::cerr << "Usage: ./ai_agent --ticker <TICKER> --query <ANALYTICS_QUESTION>" << std::endl;
        return 1;
    }

    std::string ticker = "";
    std::string user_query = "";

    // Parse CLI input strings passed by Tauri or terminal controls
    for (int i = 1; i < argc; ++i) {
        if (std::string(argv[i]) == "--ticker" && i + 1 < argc) ticker = argv[++i];
        if (std::string(argv[i]) == "--query" && i + 1 < argc) user_query = argv[++i];
    }

    // Resolve structural project paths
    fs::path binary_path = fs::absolute(argv[0]);
    fs::path build_dir = binary_path.parent_path();
    fs::path ai_root_dir = build_dir.parent_path(); 
    
    fs::path models_dir = ai_root_dir / "models";
    fs::create_directories(models_dir);
    
    fs::path model_path = models_dir / "Qwen3.5-4B-UD-Q4_K_XL.gguf";
    std::string model_path_str = model_path.string();

    // 1. Run single-pass downloader validation
    download_model_via_curl(model_path_str);

    // 2. Load financial summary layout text from disk
    std::string financial_matrix_data = load_corporate_financial_context(ai_root_dir, ticker);

    // 3. Initialize inference backend backplanes
    llama_backend_init();
    
    struct llama_model_params mparams = llama_model_default_params();
    llama_model* model = llama_model_load_from_file(model_path_str.c_str(), mparams);
    if (!model) {
        std::cerr << "❌ Critical Error: Failed to compile GGUF tensor layouts from " << model_path_str << std::endl;
        return 1;
    }

    struct llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx = 4096; // 4K Context layout footprint bounds

    // Multiprocessing compute core mapping
    unsigned int threads = std::max(2u, std::thread::hardware_concurrency() - 2);
    cparams.n_threads = threads;
    cparams.n_threads_batch = threads;

    llama_context* ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        std::cerr << "❌ Critical Error: Failed to spawn in-memory execution context window." << std::endl;
        llama_model_free(model);
        return 1;
    }

    // 4. Construct Qwen 3.5 System Chat-Template Prompt Wrapper
    std::string prompt = "<|im_start|>system\n"
                         "You are an expert corporate financial analyst agent. Below is the historical financial statement "
                         "data matrix extracted directly from the official filings for company ticker [" + ticker + "]:\n\n" 
                         + financial_matrix_data + "\n\n"
                         "Analyze these exact line items and figures mathematically to answer the user query accurately and concisely.<|im_end|>\n"
                         "<|im_start|>user\n" + user_query + "<|im_end|>\n"
                         "<|im_start|>assistant\n";

    std::cout << "\n📊 Processing Financial Data Analysis Pipeline via C++ Threads...\n" << std::endl;

    std::vector<llama_token> tokens = ::common_tokenize(ctx, prompt, true, true);

    common_params_sampling sampling_params;
    sampling_params.temp = 0.3f; // Low temperature ensures math logic consistency
    
    struct common_sampler* gui_sampler = common_sampler_init(model, sampling_params);
    if (!gui_sampler) {
        std::cerr << "❌ Critical Error: Failed to generate configuration sampler pipeline." << std::endl;
        llama_free(ctx);
        llama_model_free(model);
        return 1;
    }

    struct llama_batch batch = llama_batch_get_one(tokens.data(), tokens.size());
    const struct llama_vocab* vocab = llama_model_get_vocab(model);

    // 5. Native token streaming inference generation loop
    while (llama_decode(ctx, batch) == 0) {
        llama_token id = common_sampler_sample(gui_sampler, ctx, -1);
        
        if (llama_vocab_is_eog(vocab, id)) {
            break;
        }

        std::string token_str = common_token_to_piece(ctx, id);
        std::cout << token_str << std::flush;

        batch = llama_batch_get_one(&id, 1);
    }

    std::cout << std::endl;
    common_sampler_free(gui_sampler);
    llama_free(ctx);
    llama_model_free(model);
    llama_backend_free();

    return 0;
}