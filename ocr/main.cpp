#include <iostream>
#include <vector>
#include <string>
#include <fstream>
#include <cstdlib>
#include "llama.h"

bool download_model_file(const std::string& destination_path) {
    std::cout << "📥 Model file not found locally. Initiating direct download..." << std::endl;
    std::string download_url = "https://huggingface.co/unsloth/Qwen3.5-4B-MTP-GGUF/resolve/main/Qwen3.5-4B-UD-Q4_K_XL.gguf?download=true";
    std::string command = "curl -L -# -o " + destination_path + " \"" + download_url + "\"";
    int result = std::system(command.c_str());
    return (result == 0);
}

int main() {
    std::string model_path = "../Qwen3.5-4B-UD-Q4_K_XL.gguf";

    std::cout << "🚀 Initializing Custom Portable C++ Qwen Engine..." << std::endl;

    std::ifstream check_file(model_path);
    if (!check_file.good()) {
        if (!download_model_file(model_path)) {
            std::cerr << "❌ Download failed." << std::endl;
            return 1;
        }
    }

    llama_backend_init();

    auto mparams = llama_model_default_params();
    
    // 🛠️ MODERNIZED API FIX: Update to matching method signatures
    struct llama_model * model = llama_model_load_from_file(model_path.c_str(), mparams);
    if (!model) {
        std::cerr << "❌ Failed to load model." << std::endl;
        llama_backend_free();
        return 1;
    }

    const struct llama_vocab * vocab = llama_model_get_vocab(model);
    auto cparams = llama_context_default_params();
    cparams.n_ctx = 2048; 
    
    // 🛠️ MODERNIZED API FIX: Match initialization constructor signature
    struct llama_context * ctx = llama_init_from_model(model, cparams);
    if (!ctx) {
        std::cerr << "❌ Failed to create context layers." << std::endl;
        llama_model_free(model);
        llama_backend_free();
        return 1;
    }

    std::cout << "\n🔮 [Output Stream]:\n" << std::endl;

    // Direct text instruction
    std::string prompt = "<|user|>\nWrite a short greeting message.<|assistant|>\n";
    std::vector<llama_token> tokens(prompt.length() + 8);
    int n_tokens = llama_tokenize(vocab, prompt.c_str(), prompt.length(), tokens.data(), tokens.size(), true, true);
    tokens.resize(n_tokens);

    llama_decode(ctx, llama_batch_get_one(tokens.data(), tokens.size()));

    for (int i = 0; i < 128; ++i) {
        auto logits = llama_get_logits(ctx);
        auto n_vocab = llama_vocab_n_tokens(vocab);

        int max_token_idx = 0;
        float max_logit_val = -1e9f;
        for (int v = 0; v < n_vocab; ++v) {
            if (logits[v] > max_logit_val) {
                max_logit_val = logits[v];
                max_token_idx = v;
            }
        }

        if (llama_vocab_is_eog(vocab, max_token_idx)) {
            break;
        }

        char token_piece[64];
        int piece_len = llama_token_to_piece(vocab, max_token_idx, token_piece, sizeof(token_piece), 0, false);
        if (piece_len > 0) {
            std::cout << std::string(token_piece, piece_len) << std::flush;
        }

        llama_token next_token = max_token_idx;
        llama_decode(ctx, llama_batch_get_one(&next_token, 1));
    }

    std::cout << "\n--------------------------------------------------------" << std::endl;

    llama_free(ctx);
    llama_model_free(model);
    llama_backend_free();
    return 0;
}