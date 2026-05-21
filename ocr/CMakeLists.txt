cmake_minimum_required(VERSION 3.22)
project(OcrCli CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 🛠️ 1. Force llama.cpp to drop its example/app builds completely
set(LLAMA_BUILD_TESTS OFF CACHE INTERNAL "")
set(LLAMA_BUILD_EXAMPLES OFF CACHE INTERNAL "")
set(LLAMA_BUILD_SERVER OFF CACHE INTERNAL "")

# 📦 2. CRITICAL OPENCV MAC FIX: Turn off the broken bundled 3rdparty formats to use Apple ImageIO
set(BUILD_ZLIB OFF CACHE INTERNAL "")
set(BUILD_PNG OFF CACHE INTERNAL "")
set(BUILD_JPEG OFF CACHE INTERNAL "")
set(BUILD_TIFF OFF CACHE INTERNAL "")
set(BUILD_OPENEXR OFF CACHE INTERNAL "")
set(BUILD_WEBP OFF CACHE INTERNAL "")

set(BUILD_SHARED_LIBS OFF CACHE INTERNAL "")
set(BUILD_opencv_apps OFF CACHE INTERNAL "")
set(BUILD_opencv_tests OFF CACHE INTERNAL "")
set(BUILD_PERF_TESTS OFF CACHE INTERNAL "")

include(FetchContent)

FetchContent_Declare(
    opencv
    GIT_REPOSITORY https://github.com/opencv/opencv.git
    GIT_TAG        4.9.0
)
FetchContent_MakeAvailable(opencv)

# 📦 3. Fetch the portable llama.cpp engine core files
FetchContent_Declare(
    llamacpp
    GIT_REPOSITORY https://github.com/ggml-org/llama.cpp.git
    GIT_TAG        master
)
FetchContent_MakeAvailable(llamacpp)

# 🎯 4. Define your standalone executable target 
add_executable(OcrCli main.cpp)

# 🛠️ LINK BOTH: Connect your binary directly to the modules and append Mac's system frameworks
find_library(COCOA_LIBRARY Cocoa)
find_library(IMAGEIO_LIBRARY ImageIO)

target_link_libraries(OcrCli PRIVATE 
    llama 
    opencv_core 
    opencv_imgcodecs 
    opencv_imgproc
    ${COCOA_LIBRARY}
    ${IMAGEIO_LIBRARY}
)

# Expose header system paths for both engines
target_include_directories(OcrCli PRIVATE 
    ${llamacpp_SOURCE_DIR}/include
    ${opencv_SOURCE_DIR}/modules/core/include
    ${opencv_SOURCE_DIR}/modules/imgcodecs/include
    ${opencv_SOURCE_DIR}/modules/imgproc/include
)