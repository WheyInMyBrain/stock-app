cmake_minimum_required(VERSION 3.22)
project(ai_agent CXX)

# can initialize using its native C++17 string rules without char8_t blocks.
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

set(ARROW_BUILD_TESTS OFF CACHE INTERNAL "Force kill arrow internal tests" FORCE)
set(ARROW_BUILD_BENCHMARKS OFF CACHE INTERNAL "Force kill arrow benchmarks" FORCE)
set(ARROW_BUILD_INTEGRATION OFF CACHE INTERNAL "Force kill arrow integration checks" FORCE)
set(PARQUET_BUILD_EXECUTABLES OFF CACHE INTERNAL "Force kill parquet utility tools" FORCE)
set(PARQUET_BUILD_EXAMPLES OFF CACHE INTERNAL "Force kill example apps" FORCE)

set(LLAMA_BUILD_COMMON ON CACHE INTERNAL "Build common library sidecars" FORCE)
set(ARROW_PARQUET ON CACHE BOOL "Build Parquet support" FORCE)
set(ARROW_IPC ON CACHE BOOL "Build IPC support for Parquet schemas" FORCE)
set(ARROW_WITH_ZSTD ON CACHE BOOL "Build ZSTD compression codec support" FORCE)
set(ARROW_BUILD_STATIC ON CACHE BOOL "Build static libraries for distribution" FORCE)
set(ARROW_BUILD_SHARED OFF CACHE BOOL "Turn off shared libraries for clean distribution" FORCE)
set(ARROW_COMPUTE OFF CACHE BOOL "Turn off compute engine maps" FORCE)
set(ARROW_DATASET OFF CACHE BOOL "Turn off heavy dataset APIs" FORCE)
set(ARROW_WITH_OPENSSL OFF CACHE BOOL "Disable OpenSSL tracking completely" FORCE)
set(ARROW_WITH_THRIFT ON CACHE BOOL "Explicitly enable Thrift generation sequences" FORCE)
set(ARROW_DEPENDENCY_SOURCE "BUNDLED" CACHE STRING "Force embedded thirdparty compilation" FORCE)

include(FetchContent)

# 1. Fetch llama.cpp
FetchContent_Declare(
    llama
    GIT_REPOSITORY "https://github.com/ggml-org/llama.cpp.git"
    GIT_TAG        "master"
    GIT_SHALLOW    TRUE
)

# 2. Fetch Apache Arrow targeting the core branch explicitly
FetchContent_Declare(
    arrow_src
    GIT_REPOSITORY "https://github.com/apache/arrow.git"
    GIT_TAG        "main"          
    GIT_SHALLOW    TRUE            
    SOURCE_SUBDIR  cpp             
)

FetchContent_MakeAvailable(llama arrow_src)

# Define main entry executable target
add_executable(ai_agent src/main.cpp)

target_compile_features(ai_agent PRIVATE cxx_std_20)

# Link your portable binary natively to both llama and Arrow's compiled targets
target_link_libraries(ai_agent PRIVATE 
    llama 
    llama-common
    arrow_static
    parquet_static
)

target_include_directories(ai_agent PRIVATE 
    ${llama_SOURCE_DIR}/include 
    ${llama_SOURCE_DIR}/common
    ${arrow_src_SOURCE_DIR}/cpp/src            
    ${arrow_src_SOURCE_DIR}/cpp/src/parquet    
    ${CMAKE_BINARY_DIR}/_deps/arrow_src-build/src
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if(MSVC)
    message(STATUS "Globally adjusting MSVC compiler options to relax strict generated-code checks...")
    add_compile_options("/Od" "/bigobj" "/permissive" "/D_CRT_SECURE_NO_WARNINGS" "/wd4065" "/wd4996")
endif()