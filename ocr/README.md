
---

# 🔍 Long-Form Document OCR Subsystem (`stock-app/ocr`)

A high-performance document extraction and text processing microservice. This subsystem is responsible for taking raw, scanned PDF corporate filings (such as comprehensive Annual Reports) and transforming them into semantically structured, highly accurate Markdown documents optimized for consumption by the downstream Rust parsing engines.

---

## 🛠️ Engine Architecture & Technology Stack

The service is engineered around state-of-the-art document layout analysis frameworks to ensure complex layouts (multi-column text, data frames, accounting tables) are captured with zero data loss.

* **Layout & Structure Analyzer:** Powered by **IBM's Docling**, which provides unparalleled document understanding, semantic anchoring, and native Markdown grid reconstruction.
* **OCR Text Engine:** Backed by the highly optimized **RapidOCR** model stack, ensuring high-speed text recognition across volatile scanned page layouts.
* **Environment Virtualization:** Fully containerized via a localized `Dockerfile` to preserve layout dependencies, native C++ bindings, and CUDA acceleration support out of the box.

---

## 📂 Code Module Architecture

* **`ocr_engine.py`:** The main pipeline executor that orchestrates orchestration hooks, manages state processing, and acts as the entry CLI script.
* **`interfaces.py`:** Holds abstract data types, pipeline contracts, and internal system communication schemas.
* **`loaders.py`:** Handles disk streaming, file detection patterns, and secure local file ingestion loops.
* **`processors.py`:** Integrates the Docling context parser and manages raw structural parsing, text scrubbing, and tabular row validation.
* **`exporters.py`:** Handles encoding configurations and saves raw markdown strings straight into the directory target archive.

---

## ⚡ Direct Manual Execution

You can run the engine directly from your terminal to digitize documents for a specific company ticker.

Ensure your virtual environment is active and requirements are met (`pip install -r requirements.txt`), then execute:

```bash
# General Syntax: python ocr_engine.py <TICKER>
python ocr_engine.py IMFA

```

This commands will scan your raw storage pools for any matching PDFs under the `IMFA` footprint, execute the layout parsing pass, and deposit the final text files cleanly into the downstream directory path structure:
`stock-app/data/IMFA/ocr/annual-reports/*.md`

---

## 🐳 Containerized Deployment

To process pipelines inside an isolated Docker ecosystem without configuring native system dependencies:

```bash
# Build the container asset
docker build -t stock-app-ocr .

# Spin up processing tasks inside the container footprint
docker run -v $(pwd)/../data:/app/data stock-app-ocr python ocr_engine.py IMFA

```