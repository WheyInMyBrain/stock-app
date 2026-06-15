#pragma once
#include <string>
#include <sstream>
#include <algorithm>
#include "ai_engine.hpp"
#include "metrics_config.hpp"

class AIRouter {
private:
    std::string build_gbnf_grammar() {
        std::string gbnf = "root ::= ";
        size_t i = 0;
        for (const auto& [tag, _] : MetricRegistry) {
            gbnf += "\"" + tag + "\"";
            if (i < MetricRegistry.size() - 1) gbnf += " | ";
            i++;
        }
        return gbnf;
    }

    std::string build_schema_description() {
        std::string context = "Available Metric Schema Options:\n";
        for (const auto& [tag, meta] : MetricRegistry) {
            context += "- Tag: [" + tag + "] -> Focuses on: " + meta.ai_description + "\n";
        }
        return context;
    }

public:
    std::string resolve_intent_tag(AIEngine& engine, const std::string& user_query) {
        llama_context* ctx = engine.get_ctx();
        llama_model* model = engine.get_model();

        std::string prompt = "<|im_start|>system\n"
                             "You are a strategic financial router mapping user queries to corporate metric tags.\n" 
                             + build_schema_description() + "\n\n"
                             "Analyze the user's intent and select the single absolute matching classification tag.<|im_end|>\n"
                             "<|im_start|>user\n" + user_query + "<|im_end|>\n"
                             "<|im_start|>assistant\n";

        std::vector<llama_token> tokens = ::common_tokenize(ctx, prompt, true, true);

        common_params_sampling sampling_params;
        sampling_params.temp = 0.0f; // Pure mathematical consistency
        sampling_params.grammar = common_grammar(COMMON_GRAMMAR_TYPE_USER, build_gbnf_grammar());

        struct common_sampler* gui_sampler = common_sampler_init(model, sampling_params);
        struct llama_batch batch = llama_batch_get_one(tokens.data(), tokens.size());
        const struct llama_vocab* vocab = llama_model_get_vocab(model);

        std::string extracted_tag = "";
        int safety_counter = 0;

        while (llama_decode(ctx, batch) == 0 && safety_counter < 32) {
            llama_token id = common_sampler_sample(gui_sampler, ctx, -1);
            if (llama_vocab_is_eog(vocab, id)) break;

            std::string token_str = common_token_to_piece(ctx, id);
            extracted_tag += token_str;
            safety_counter++;

            common_sampler_accept(gui_sampler, id, true);
            batch = llama_batch_get_one(&id, 1);
        }

        common_sampler_free(gui_sampler);
        extracted_tag.erase(std::remove_if(extracted_tag.begin(), extracted_tag.end(), ::isspace), extracted_tag.end());
        return extracted_tag;
    }
};