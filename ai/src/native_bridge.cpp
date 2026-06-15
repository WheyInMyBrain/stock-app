#include "native_bridge.hpp"
#include "ai_engine.hpp"
#include "ai_router.hpp"
#include "ai_analyst.hpp"
#include "metrics_config.hpp"
#include <string>
#include <cstring>

extern "C" {

// 1. Step 1 Bridge: Uses AIRouter exactly like main.cpp
const char* bridge_resolve_intent(const char* model_path, const char* user_query) {
    if (!model_path || !user_query) return strdup("ERROR_INVALID_INPUT");

    AIEngine engine;
    if (!engine.initialize(model_path)) return strdup("ERROR_INIT_FAILED");

    AIRouter router;
    std::string outcome_tag = router.resolve_intent_tag(engine, user_query);
    return strdup(outcome_tag.c_str());
}

// 2. Step 2 Bridge: Uses MetricRegistry and your core layout functions
const char* bridge_extract_matrix(const char* data_dir, const char* ticker, const char* intent_tag) {
    if (!data_dir || !ticker || !intent_tag) return strdup("ERROR_INVALID_INPUT");

    auto it = MetricRegistry.find(intent_tag);
    if (it == MetricRegistry.end()) return strdup("ERROR_TAG_NOT_FOUND");

    std::string matrix_output = it->second.run_function(data_dir, ticker, it->second);
    return strdup(matrix_output.c_str());
}

// 3. Step 3 Bridge: Instantiates your exact AIAnalyst logic with the callback channel
void bridge_stream_synthesis(
    const char* model_path, 
    const char* user_query, 
    const char* ticker, 
    const char* data_matrix,
    TokenCallback callback
) {
    if (!model_path || !user_query || !ticker || !data_matrix || !callback) return;

    AIEngine engine;
    if (!engine.initialize(model_path)) return;

    // Direct invocation of your native analyst module file
    AIAnalyst analyst;
    analyst.generate_explanation(engine, user_query, ticker, data_matrix, callback);
}

void free_bridge_string(const char* ptr) {
    if (ptr) {
        free((void*)ptr);
    }
}

}