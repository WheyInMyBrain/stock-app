#pragma once
#include <string>

namespace Ocr {
namespace Storage {

class ModelInstaller {
public:
    // Ensures model paths exist on disk; triggers network pulls if missing
    static void validate_and_provision_weights(const std::string& destination_path);
};

} // namespace Storage
} // namespace Ocr