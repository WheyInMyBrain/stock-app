
# Stock App

A comprehensive, multi-language financial data extraction and intrinsic valuation platform. This system handles everything from fetching raw web data and running OCR on annual reports to executing complex financial models and presenting them in an interactive user interface.

## 🛠 Prerequisites

This project utilizes a polyglot architecture. Ensure you have the following environments installed on your system before building:

* **Rust** (Powers the core engine, UI, parser, and main downloader)
* **Go** (Handles client initialization and network sessions)
* **Python** (Powers the OCR extraction pipeline)

## 📁 Project Architecture

The repository is modularized by function and language:

* **`ui/`** - The interactive application layer.
    * **`frontend/`** - The frontend user interface workspace.
    * **`backend/`** - The bridge connecting the UI to the local data pools and calculation engines.
* **`analysis/`** - The core mathematical engine. Executes stateless, on-the-fly intrinsic valuation models (including DCF, DDM, Residual Income, EPV, Benjamin Graham, and EVA).
* **`downloader-rust/`** - The primary network worker. Connects to various financial websites to scrape and download raw market data.
* **`downloader-go/`** - A specialized Go module handling HTTP client initialization, session management, and cookie handling for secure external connections.
* **`ocr/`** - A Python-based Optical Character Recognition engine designed to scan annual reports and extract raw financial text.
* **`parser/`** - The data sanitation pipeline. It takes all the raw downloaded data and messy OCR output, then cleans, formats, and structures it into a unified schema for the analysis engine to use.
* **`data/`** - *(Dynamically Generated)* This directory is created automatically at runtime. It stores organized, human-readable data folders separated by individual stock tickers, allowing users to inspect the raw historical data manually if needed.

## 🚀 Compilation & Execution

The entire application is compiled and launched directly from the frontend directory using Rust's package manager.

1. Navigate to the frontend workspace:
```bash

cd stock-app/ui/frontend

```

2. Compile and run the application:
```bash

cargo run

```