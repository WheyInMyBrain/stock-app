# 🏛️ Ingestion & Parsing Engine (`stock-app/parser`)

A high-performance data normalization and serialization engine built in Rust. This subsystem processes raw, unstructured corporate filings downloaded directly from the official Bombay Stock Exchange (BSE) and National Stock Exchange (NSE) portals, sanitizes their semantic structures, and serializes them into optimized Apache Parquet tables within the centralized data storage footprint.

---

## ⚡ Core Operational Pipeline

```
┌────────────────────────┐      ┌────────────────────────┐      ┌────────────────────────┐
│ Raw Downloaded Files   │ ───> │  Rust Parser Engine   │ ───> │ Immutable Parquet Data │
│ (HTML/JSON/XML Layout) │      │ (Sanitation & Pivot)   │      │ (../data/<TICKER>/)    │
└────────────────────────┘      └────────────────────────┘      └────────────────────────┘

```

1. **Ingestion Loop:** Scans raw directory trees containing corporate reports downloaded from exchange servers.
2. **Sanitation & Alignment:** Strips text anomalies, normalizes varied accounting terminology schemas, and structures metadata footprints (such as date boundaries and reporting periods).
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

### 🚀 Core System Orchestrators

* **`main.rs`:** The central CLI wrapper that orchestrates parsing sweeps by accepting command parameters and processing targets.
* **`bse_parser.rs` & `nse_parser.rs`:** Root handlers that load raw files from disk and delegate tracking states to their respective exchange submodules.
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
└── nse_corporate-shareholding-master.parquet

```

> **Performance Note:** Converting row-based web formats into structured Parquet tables shrinks storage footprints by up to 80% while enabling zero-copy memory mapping inside downstream analytical cores.