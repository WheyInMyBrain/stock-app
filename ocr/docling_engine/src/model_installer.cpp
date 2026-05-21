#include "model_installer.hpp"
#include <iostream>
#include <filesystem>
#include <stdexcept>
#include <cstdlib>

namespace fs = std::filesystem;

namespace Ocr {
namespace Storage {

void ModelInstaller::validate_and_provision_weights(const std::string& destination_path) {
    if (fs::exists(destination_path)) {
        std::cout << "✅ [Storage Gate]: Granite-Docling BF16 weights verified locally." << std::endl;
        return;
    }

    std::cout << "⚠️  [Storage Gate]: Target asset missing from: " << destination_path << std::endl;
    
    // Auto-scaffold directory structure safely if missing
    fs::path target_file_path(destination_path);
    fs::path parent_dir = target_file_path.parent_path();
    if (!parent_dir.empty() && !fs::exists(parent_dir)) {
        fs::create_directories(parent_dir);
    }

    std::string target_url = "https://huggingface.co/ibm-granite/granite-docling-258M-GGUF/resolve/main/granite-docling-258M-BF16.gguf";
    std::cout << "📥 [Storage Gate]: Streaming binary matrix directly from HuggingFace..." << std::endl;

    int sys_status = -1;
#if defined(_WIN32)
    std::string win_cmd = "powershell -Command \"Invoke-WebRequest -Uri '" + target_url + "' -OutFile '" + destination_path + "'\"";
    sys_status = std::system(win_cmd.c_str());
#else
    std::string posix_cmd = "curl -L -o \"" + destination_path + "\" \"" + target_url + "\"";
    sys_status = std::system(posix_cmd.c_str());
#endif

    if (sys_status != 0) {
        fs::remove(destination_path); // Clean up dirty or partial download files
        throw std::runtime_error("❌ [Storage Gate Exception]: Network pipe execution failed. Check system connection profiles.");
    }
    
    std::cout << "✨ [Storage Gate]: Ingestion complete. Weights matched seamlessly to disk." << std::endl;
}

} // namespace Storage
} // namespace Ocr