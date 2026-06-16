cmake_minimum_required(VERSION 3.22)
project(ai_agent CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

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

add_executable(ai_agent src/main.cpp)

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
    message(STATUS "Globally throttling MSVC compiler options to fit within runner RAM bounds...")
    add_compile_options("/Od" "/bigobj")
endif()