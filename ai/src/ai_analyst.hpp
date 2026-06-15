#pragma once
#include <string>
#include <vector>
#include "ai_engine.hpp"

class AIAnalyst {
public:
    std::string generate_explanation(AIEngine& engine, const std::string& user_query, const std::string& ticker, const std::string& data_matrix) {
        llama_context* ctx = engine.get_ctx();
        llama_model* model = engine.get_model();

        // Standard Qwen/ChatML prompt packing sequence
        std::string prompt = "<|im_start|>system\n"
                             "You are an expert corporate financial analyst. Below is the raw data extracted from "
                             "exchange filings for ticker symbol: " + ticker + ".\n\n"
                             "[EXTRACTED TIME-SERIES DATA]\n" + data_matrix + "\n\n"
                             "Using only the provided dataset, answer the user's inquiry clearly and directly. "
                             "Point out specific dates, highlight trend shifts, and answer any analytical "
                             "questions asked without over-complicating or making up data outside the scope.<|im_end|>\n"
                             "<|im_start|>user\n" + user_query + "<|im_end|>\n"
                             "<|im_start|>assistant\n";

        std::vector<llama_token> tokens = ::common_tokenize(ctx, prompt, true, true);

        common_params_sampling sampling_params;
        sampling_params.temp = 0.4f; // Balanced for concise, natural explanation delivery
        
        struct common_sampler* analyst_sampler = common_sampler_init(model, sampling_params);
        struct llama_batch batch = llama_batch_get_one(tokens.data(), tokens.size());
        const struct llama_vocab* vocab = llama_model_get_vocab(model);

        std::string text_explanation = "";
        int safety_counter = 0;
        int max_tokens = 1024;

        while (llama_decode(ctx, batch) == 0 && safety_counter < max_tokens) {
            llama_token id = common_sampler_sample(analyst_sampler, ctx, -1);
            if (llama_vocab_is_eog(vocab, id)) break;

            std::string token_str = common_token_to_piece(ctx, id);
            text_explanation += token_str;
            
            // Print the response in real-time as the hardware generates tokens
            std::cout << token_str << std::flush;
            
            safety_counter++;
            common_sampler_accept(analyst_sampler, id, true);
            batch = llama_batch_get_one(&id, 1);
        }

        common_sampler_free(analyst_sampler);
        return text_explanation;
    }
};