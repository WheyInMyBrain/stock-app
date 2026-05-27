cmake_minimum_required(VERSION 3.22)
project(ai_agent CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# =========================================================================
# ⚙️ LLAMA.CPP OPTIONS (FORCES EXPORTING THE ENTIRE COMMON STRUCT)
# =========================================================================
set(LLAMA_BUILD_COMMON ON CACHE INTERNAL "Build common library for sidecar utilities" FORCE)

# =========================================================================
# 🎯 AUTOMATED DEPENDENCY INGESTION VIA FETCHCONTENT
# =========================================================================
include(FetchContent)

message(STATUS "Configuring automated llama.cpp upstream dependency...")

FetchContent_Declare(
    llama
    GIT_REPOSITORY "https://github.com/ggml-org/llama.cpp.git"
    GIT_TAG        "master"
    GIT_SHALLOW    TRUE     # Shallow clone to minimize download size
    GIT_PROGRESS   TRUE     # Stream download metrics to console
)

FetchContent_MakeAvailable(llama)

# =========================================================================
# 🛠️ EXECUTABLE TARGET AND DEPENDENCY CORRELATION
# =========================================================================
add_executable(ai_agent src/main.cpp)

# 🚀 FIX: Link directly to 'llama-common' instead of 'common'
# This establishes a strict build sequence order for the parallel compiler!
target_link_libraries(ai_agent PRIVATE llama llama-common)

# Expose required internal build directories dynamically
target_include_directories(ai_agent PRIVATE 
    ${llama_SOURCE_DIR}/include 
    ${llama_SOURCE_DIR}/common
)