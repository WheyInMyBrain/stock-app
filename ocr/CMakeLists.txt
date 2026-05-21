cmake_minimum_required(VERSION 3.22)
project(OcrCli CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 🛠️ Disable buggy internal examples/apps before fetching
set(LLAMA_BUILD_TESTS OFF CACHE INTERNAL "")
set(LLAMA_BUILD_EXAMPLES OFF CACHE INTERNAL "")
set(LLAMA_BUILD_SERVER OFF CACHE INTERNAL "")

include(FetchContent)
FetchContent_Declare(
    llamacpp
    GIT_REPOSITORY https://github.com/ggml-org/llama.cpp.git
    GIT_TAG        master
)
FetchContent_MakeAvailable(llamacpp)

add_executable(OcrCli main.cpp)

# 🛠️ Purely link against the core framework target and your local OpenCV
target_link_libraries(OcrCli PRIVATE llama ${OpenCV_LIBS})

target_include_directories(OcrCli PRIVATE ${llamacpp_SOURCE_DIR}/include)