#pragma once
#include <string>
#include <filesystem>
#include <memory>
#include <arrow/io/file.h>
#include <parquet/arrow/reader.h>
#include <arrow/table.h>
#include <arrow/result.h>

namespace fs = std::filesystem;

class DataLoader {
public:
    static arrow::Result<std::shared_ptr<arrow::Table>> load_parquet_to_table(const std::string& file_path) {
        if (!fs::exists(file_path)) {
            return arrow::Status::IOError("Local target Parquet file is missing: " + file_path);
        }

        // 1. Open local file streaming layer
        ARROW_ASSIGN_OR_RAISE(auto infile, arrow::io::ReadableFile::Open(file_path));

        // 2. Instantiate native file reader context
        std::unique_ptr<parquet::arrow::FileReader> reader;
        ARROW_ASSIGN_OR_RAISE(reader, parquet::arrow::OpenFile(infile, arrow::default_memory_pool()));

        // 3. Extract and return unified Table chunk layout
        std::shared_ptr<arrow::Table> table;
        ARROW_ASSIGN_OR_RAISE(table, reader->ReadTable());

        return table;
    }
};