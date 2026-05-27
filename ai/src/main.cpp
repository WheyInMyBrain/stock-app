#include <iostream>
#include <string>
#include <vector>
#include <filesystem>
#include <cstdlib>
#include <thread>
#include <algorithm>
#include "llama.h"
#include "common.h"
#include "sampling.h"

namespace fs = std::filesystem;

// Background downloader that targets the absolute local folder path
void download_model_via_curl(const std::string& target_path) {
    if (fs::exists(target_path)) return;

    std::cout << "🤖 First launch initializing. Downloading Qwen 3.5 Core (2.99 GB)..." << std::endl;
    std::cout << "📍 Destination: " << target_path << std::endl;
    
    // Direct link to the high-performance unsloth Q4_K_XL model checkpoint
    std::string url = "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-UD-Q4_K_XL.gguf";
    std::string cmd = "curl -L -o \"" + target_path + "\" " + url;
    
    int result = std::system(cmd.c_str());
    if (result != 0) {
        std::cerr << "❌ System Error: Execution layer failed to stream file via curl command." << std::endl;
        std::exit(1);
    }
    std::cout << "✅ Model checkpoint successfully downloaded and cached inside the project!" << std::endl;
}

int main(int argc, char* argv[]) {
    // 🎯 RESOLVE LOCAL WORKSPACE PATH: 
    // Find where the compiled binary is executing (e.g., stock-app/ai/build/ai_agent)
    fs::path binary_path = fs::absolute(argv[0]);
    fs::path build_dir = binary_path.parent_path();
    fs::path ai_root_dir = build_dir.parent_path(); // Backs out of 'build/' to reach 'ai/' folder root
    
    // Create 'models/' directly inside the stock-app/ai/ folder directory footprint
    fs::path models_dir = ai_root_dir / "models";
    fs::create_directories(models_dir);
    
    fs::path model_path = models_dir / "Qwen3.5-4B-UD-Q4_K_XL.gguf";
    std::string model_path_str = model_path.string();

    // 1. Ensure the model file is pulled directly to stock-app/ai/models/
    download_model_via_curl(model_path_str);

    // 2. Initialize inference backend engines
    llama_backend_init();
    
    struct llama_model_params mparams = llama_model_default_params();
    llama_model* model = llama_model_load_from_file(model_path_str.c_str(), mparams);
    if (!model) {
        std::cerr << "❌ Critical Error: Failed to compile GGUF tensor layouts from " << model_path_str << std::endl;
        return 1;
    }

    struct llama_context_params cparams = llama_context_default_params();
    cparams.n_ctx = 2048;

    unsigned int threads = std::max(2u, std::thread::hardware_concurrency() - 2);
    cparams.n_threads = threads;
    cparams.n_threads_batch = threads;

    llama_context* ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        std::cerr << "❌ Critical Error: Failed to spawn in-memory execution context window." << std::endl;
        llama_model_free(model);
        return 1;
    }

    // 3. Assemble confirmation prompt sequence
    std::string prompt = "<|im_start|>system\nYou are a concise financial chatbot.<|im_end|>\n"
                         "<|im_start|>user\nHello Qwen! Please tell me a quick 'hi' and confirm you are ready to analyze my Parquet database tables.<|im_end|>\n"
                         "<|im_start|>assistant\n";

    std::cout << "\n🚀 Processing input prompt matrix locally via C++..." << std::endl;

    std::vector<llama_token> tokens = ::common_tokenize(ctx, prompt, true, true);

    common_params_sampling sampling_params;
    sampling_params.temp = 0.4f;
    
    struct common_sampler* gui_sampler = common_sampler_init(model, sampling_params);
    if (!gui_sampler) {
        std::cerr << "❌ Critical Error: Failed to generate configuration sampler pipeline." << std::endl;
        llama_free(ctx);
        llama_model_free(model);
        return 1;
    }

    struct llama_batch batch = llama_batch_get_one(tokens.data(), tokens.size());
    const struct llama_vocab* vocab = llama_model_get_vocab(model);

    // 4. In-process execution inference loop
    while (llama_decode(ctx, batch) == 0) {
        llama_token id = common_sampler_sample(gui_sampler, ctx, -1);
        
        if (llama_vocab_is_eog(vocab, id)) {
            break;
        }

        std::string token_str = common_token_to_piece(ctx, id);
        std::cout << token_str << std::flush;

        batch = llama_batch_get_one(&id, 1);
    }

    // 5. Gracefully clear allocation references from memory
    std::cout << std::endl;
    common_sampler_free(gui_sampler);
    llama_free(ctx);
    llama_model_free(model);
    llama_backend_free();

    return 0;
}