cmake_minimum_required(VERSION 3.16)

# Compile both structural files into the same modular engine library
add_library(docling_vlm_engine STATIC
    src/docling_vlm.cpp
    src/model_installer.cpp
)

target_include_directories(docling_vlm_engine PUBLIC include)

target_include_directories(docling_vlm_engine PRIVATE 
    ${llamacpp_backend_SOURCE_DIR}/include
    ${llamacpp_backend_SOURCE_DIR}
)

target_link_libraries(docling_vlm_engine PUBLIC llama)