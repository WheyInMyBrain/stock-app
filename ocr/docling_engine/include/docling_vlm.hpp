#pragma once
#include <string>
#include <memory>

namespace Ocr {
namespace Engine {

class DoclingVlm {
public:
    // Pass the disk path pointing to your quantized granite-docling GGUF model
    explicit DoclingVlm(const std::string& model_path);
    ~DoclingVlm();

    // Ingests a raw page (image/extracted frame bytes) and extracts a structured Markdown table grid
    std::string parse_financial_sheet(const std::string& image_path, const std::string& command_prompt);

private:
    // Implementation class pointer pattern to prevent leakage of third-party structures into public scopes
    class Impl;
    std::unique_ptr<Impl> pImpl;
};

} // namespace Engine
} // namespace Ocr