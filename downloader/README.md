# 🌐 Automated Data Downloader Engine (`stock-app/downloader`)

A high-concurrency network ingestion layer built in Go (Golang). This subsystem is responsible for authenticating with the backend API servers of the Bombay Stock Exchange (BSE) and National Stock Exchange (NSE), handling security headers, managing cookie sessions, and downloading raw corporate reports and historical market data into the central data architecture.

---

## ⚡ Core Network Pipeline

```
┌────────────────────────┐      ┌────────────────────────┐      ┌────────────────────────┐
│  BSE / NSE API Portals │ ───> │   Go Network Engine    │ ───> │  Raw Unstructured Data │
│ (Session Handshakes)   │      │  (Concurrent Workers)  │      │  (../data/<TICKER>/)   │
└────────────────────────┘      └────────────────────────┘      └────────────────────────┘

```

1. **Session Handshake:** Generates dynamic headers, rotates user-agent strings, and executes token exchanges to pass through exchange security protocols.
2. **Endpoint Polling:** Queries undocumented exchange backend API nodes to fetch precise resource locators for financial results, shareholding matrices, and historical charts.
3. **Streamed Disk Dumps:** Downloads file streams concurrently and commits them to disk inside ticker-specific tracking directories.

---

## 📂 Submodule Directory Architecture

### 🏛️ The BSE Scraping Engine (`scrape_bse/`)

Handles requests directed at the BSE corporate tracking systems.

* **`client.go` & `endpoints.go`:** Manages the baseline HTTP connection pool and translates parameters into internal BSE API endpoint strings.
* **`download.go` & `lookup.go`:** Executes target file transfers and maps public stock tickers to exchange-specific corporate identifiers.
* **`pipeline.go`:** Orchestrates the automated tracking loops to pull fresh historical files in parallel.

### 📈 The NSE Scraping Engine (`scrape_nse/`)

Handles the cookies and stateful tracking loops required to navigate NSE backend networks.

* **`api.go` & `client.go`:** Configures session cookies and security tokens required to bypass active request blocks.
* **`endpoints.go` & `download.go`:** Fetches continuous financial tables, corporate statements, and time-series historical chart logs (`10Y.json`).
* **`pipeline.go`:** Distributes network tasks across lightweight Go-routines for high-speed batch asset downloading.

### 🚀 Core System Orchestrator

* **`main.go`:** The centralized execution framework that reads terminal input tickers and boots up the concurrent BSE/NSE ingestion streams.

---

## 💾 Output Storage Target Infrastructure

Files downloaded by this engine serve as the absolute baseline truth data layer for the downstream Rust parsing and analytical subsystems. Collected records are saved into the central data footprint using this raw organizational schema:

```text
../data/<TICKER>/
├── bse_historical-chart-data/
│   └── 10Y.json
├── nse_historical-chart-data/
│   └── 10Y.json
└── <RAW_EXCHANGE_DOCUMENTS>/
    ├── financial-results-raw_files/
    └── shareholding-pattern-raw_files/

```