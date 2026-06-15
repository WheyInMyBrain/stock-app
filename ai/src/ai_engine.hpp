#pragma once
#include <string>
#include <vector>
#include <filesystem>
#include <thread>
#include <algorithm>
#include <iostream>
#include <cstdlib>
#include "llama.h"
#include "common.h"
#include "sampling.h"

namespace fs = std::filesystem;

class AIEngine {
private:
    llama_model* model = nullptr;
    llama_context* ctx = nullptr;

public:
    ~AIEngine() {
        if (ctx) llama_free(ctx);
        if (model) llama_model_free(model);
        llama_backend_free();
    }

    void verify_and_download_model(const std::string& target_path) {
        fs::path p(target_path);
        fs::path parent_dir = p.parent_path();

        // Ensure the directory exists using absolute pathing references
        if (!fs::exists(parent_dir)) {
            std::cout << "📂 Creating absolute models cache directory at: " << fs::absolute(parent_dir) << std::endl;
            std::error_code ec;
            fs::create_directories(parent_dir, ec);
            if (ec) {
                std::cerr << "❌ OS Filesystem Error: Failed to create directories: " << ec.message() << std::endl;
                std::exit(1);
            }
        }

        if (fs::exists(p)) return;

        std::cout << "🤖 First launch initializing. Downloading Qwen 3.5 Core (2.99 GB)..." << std::endl;
        std::cout << "📍 Absolute Destination: " << fs::absolute(p) << std::endl;
        
        std::string url = "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-UD-Q4_K_XL.gguf";
        
        // Escape quotes cleanly for standard shell command execution wrappers
        std::string cmd = "curl -L -o \"" + fs::absolute(p).string() + "\" \"" + url + "\"";
        
        std::cout << "🚀 Running download command..." << std::endl;
        int result = std::system(cmd.c_str());
        if (result != 0) {
            std::cerr << "❌ System Error: Model streaming layout download failed." << std::endl;
            std::exit(1);
        }
        std::cout << "✅ Model download complete!" << std::endl;
    }

    bool initialize(const std::string& model_path_str) {
        llama_backend_init();
        struct llama_model_params mparams = llama_model_default_params();
        model = llama_model_load_from_file(model_path_str.c_str(), mparams);
        if (!model) return false;

        struct llama_context_params cparams = llama_context_default_params();
        cparams.n_ctx = 4096; 

        unsigned int threads = std::max(2u, std::thread::hardware_concurrency() - 2);
        cparams.n_threads = threads;
        cparams.n_threads_batch = threads;

        ctx = llama_init_from_model(model, cparams);
        return ctx != nullptr;
    }

    llama_model* get_model() { return model; }
    llama_context* get_ctx() { return ctx; }
};