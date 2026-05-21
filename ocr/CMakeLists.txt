cmake_minimum_required(VERSION 3.16)
project(StockAppOcrCore CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Force optimizations for heavy matrix operations
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -march=native")

# ==============================================================================
# 📦 STEP 1: PULL NATIVE LLAMA.CPP BACKEND AUTONOMOUSLY
# ==============================================================================
include(FetchContent)

FetchContent_Declare(
    llamacpp_backend
    GIT_REPOSITORY https://github.com/ggerganov/llama.cpp.git
    GIT_TAG        master  # Pulls the stable core supporting newer SigLIP2/Idefics3 VLM architectures
)

FetchContent_MakeAvailable(llamacpp_backend)

# ==============================================================================
# 🗂️ STEP 2: CASCADE TO MODULAR SUBSYSTEM DIRECTORIES
# ==============================================================================
# Add our isolated engine directory to the build tree
add_subdirectory(docling_engine)

# Construct the root system command line executor
add_executable(OcrCli src/main.cpp)

# Link the executable directly against our abstracted sub-module library
target_link_libraries(OcrCli PRIVATE docling_vlm_engine)