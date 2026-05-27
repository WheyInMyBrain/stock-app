
---

# 🏛️ Ingestion & Parsing Engine (`stock-app/parser`)

A high-performance data normalization and serialization engine built in Rust. This subsystem processes raw, unstructured corporate filings downloaded directly from the official Bombay Stock Exchange (BSE) and National Stock Exchange (NSE) portals, alongside long-form unstructured OCR documents, sanitizing their semantic structures and serializing them into optimized Apache Parquet tables within the centralized data storage footprint.

---

## ⚡ Core Operational Pipeline

```
┌────────────────────────┐      ┌────────────────────────┐      ┌────────────────────────┐
│ Raw Downloaded Files   │ ───> │  Rust Parser Engine    │ ───> │ Immutable Parquet Data │
│ (HTML/JSON/XML/MD/OCR) │      │ (Sanitation & Pivot)   │      │ (../data/<TICKER>/)    │
└────────────────────────┘      └────────────────────────┘      └────────────────────────┘

```

1. **Ingestion Loop:** Scans raw directory trees containing exchange JSONs or text-heavy markdown documents extracted via OCR engines.
2. **Sanitation & Alignment:** Strips text anomalies, applies fallback financial barrier logic to bypass empty header row truncation, and normalizes word stuttering or text duplication artifacts across raw cells.
3. **Parquet Serialization:** Writes compressed columnar binary files (`.parquet`) right into target asset folders for sub-millisecond querying by the downstream analytics engines.

---

## 📂 Submodule Directory Architecture

### 🏛️ The BSE Pipeline (`src/bse/`)

Contains modules designed to navigate the unstructured reporting layouts unique to the Bombay Stock Exchange.

* **`financial_report.rs`:** Ingests raw corporate earnings results, aligning varying balancing line items into structured matrix nodes.
* **`shareholding.rs`:** Processes equity ownership structures, backing out insider, promoter, institutional, and retail allocation layers.
* **`utils.rs` & `mod.rs`:** Handles string normalizations, currency conversions, and module interface packaging.

### 📈 The NSE Pipeline (`src/nse/`)

Contains modules mapped to the data streams of the National Stock Exchange.

* **`financial_report.rs`:** Parses and pivots annual and interim tabular financial disclosures.
* **`shareholding.rs`:** Unpacks tracking metrics across promoter groupings, Foreign Portfolio Investors (FPI/FII), Domestic Institutions (DII), and retail accounts.
* **`corporate_governance.rs`:** Extracts board composition arrays and listing compliance markers.
* **`investor_complaints.rs`:** Tracks red-flag grievance resolution vectors.
* **`utils.rs` & `mod.rs`:** Houses specialized parsing lookups unique to NSE corporate disclosure structures.

### 🔎 The Unstructured OCR Pipeline (`src/ocr/`)

Contains extraction submodules optimized to strip structural layout discrepancies out of scanned and digitized Annual Reports.

* **`balance_sheet.rs`:** Anchors onto balance sheet heading targets, identifying structural notes column metrics and resolving tabular padding issues.
* **`revenue.rs`:** Targets operational text domains (Profit & Loss statements) to isolate itemized cost and revenue metrics into standardized timelines.
* **`cash_flow.rs`:** Captures raw financial sequences under statement headers without column truncation, stripping out duplicate word artifacts via an anti-stuttering loop.
* **`ocr_parser.rs` & `utils.rs`:** Implements common traits for parsing grids, handling layout string sanitization, and routing tables into the shared processing vector stack.

### 🚀 Core System Orchestrators

* **`main.rs`:** The central CLI wrapper that orchestrates parsing sweeps by accepting command parameters and processing targets (e.g., `cargo run --release IMFA`).
* **`bse_parser.rs`, `nse_parser.rs` & `ocr_parser.rs`:** Root handlers that load raw files from disk and delegate tracking states to their respective exchange or OCR submodules.
* **`targets.rs`:** Manages the task scheduling matrix, determining which company tickers require active parsing updates.
* **`utils.rs`:** Shared, global utility primitives for filesystem handling and error logging.

---

## 💾 Output Framework Data Targets

The engine bypasses raw database writes, flashing processed metrics straight into highly scannable columnar files. For any active asset, outputs are written into the centralized data footprint layout:

```text
../data/<TICKER>/parquets/
├── bse_financial-results-docs.parquet
├── bse_shareholding-pattern-docs.parquet
├── nse_corporates-financial-results.parquet
├── nse_corporate-shareholding-master.parquet
└── annual_report/
    ├── balance_sheet.parquet
    ├── income_statement.parquet
    └── cash_flow.parquet

```

> **Performance Note:** Converting row-based web formats and sparse OCR tables into structured Parquet tables shrinks storage footprints by up to 80% while enabling zero-copy memory mapping inside downstream analytical cores.