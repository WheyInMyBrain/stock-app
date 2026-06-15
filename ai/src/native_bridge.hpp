#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// A functional bridge pointer mapping generated words directly out to the Rust runtime
typedef void (*TokenCallback)(const char* token_text);

// 1. Step 1 Bridge Function: Returns pure tag intent text 
const char* bridge_resolve_intent(const char* model_path, const char* user_query);

// 2. Step 2 Bridge Function: Merges files and returns the raw matching matrix data sheet
const char* bridge_extract_matrix(const char* data_dir, const char* ticker, const char* intent_tag);

// 3. Step 3 Bridge Function: Streams token strings back via callback execution pointers
void bridge_stream_synthesis(
    const char* model_path, 
    const char* user_query, 
    const char* ticker, 
    const char* data_matrix,
    TokenCallback callback
);

// Memory tracking clean-up layer
void free_bridge_string(const char* ptr);

#ifdef __cplusplus
}
#endif