#include <iostream>
#include <filesystem>
#include "docling_vlm.hpp"

namespace fs = std::filesystem;

int main(int argc, char* argv[]) {
    if (argc < 3) {
        std::cout << "Usage: ./OcrCli <Target_Ticker> <Exchange_Label>" << std::endl;
        return 1;
    }

    std::string ticker = argv[1];
    std::string exchange = argv[2];

    std::string model_file_path = "models/granite-docling-258m-bf16.gguf";
    std::string document_image_proxy = "../data/" + ticker + "/raw_files/latest_financials.png";

    try {
        // Instantiate the isolated sub-engine module abstractly
        Ocr::Engine::DoclingVlm docling_core(model_file_path);

        // Define a strict processing prompt to enforce structured markdown tables
        std::string operational_prompt = "Convert this page table to clean markdown.";

        std::string completed_matrix = docling_core.parse_financial_sheet(document_image_proxy, operational_prompt);

        std::cout << "\n🎯 [SUCCESSFUL EXTRACTION VECTOR]:\n" << completed_matrix << std::endl;

    } catch (const std::exception& e) {
        std::cerr << "Runtime Exception Intercepted: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}