# 🚀 Multi-Exchange Financial Analysis Engine (NSE & BSE)

A high-performance, low-memory financial data mining platform written in **Go**. The application concurrently extracts data strings, real-time volume matrices, multi-timeframe charts, and physical raw corporate regulatory filing assets (`.json`, `.xml`, `.html`, `.pdf`) directly from the backend endpoints of both the **National Stock Exchange (NSE)** and the **Bombay Stock Exchange (BSE)**.

---

## 🏗️ System Architecture

The core philosophy of this workspace is **strict exchange isolation**. By splitting logic into `scrape_nse` and `scrape_bse` modules, network failures, DOM shifts, or API contract updates on one exchange can never cause a crash or regression in the other.

```text
stock-app/
├── data/                       <-- Extracted local asset storage (Git-ignored)
├── downloader/
│   ├── main.go                 <-- Core orchestration & command-line router
│   ├── scrape_nse/             <-- National Stock Exchange Engine
│   │   ├── client.go           <-- Cookies & automated session handshaker
│   │   ├── download.go         <-- Low-RAM file-streaming worker logic
│   │   ├── endpoints.go        <-- Registered NSE extraction strategies (14 APIs)
│   │   └── pipeline.go         <-- Sequential strategy loop manager
│   └── scrape_bse/             <-- Bombay Stock Exchange Engine
│       ├── client.go           <-- Cross-domain TLS & handshake configurations
│       ├── lookup.go           <-- Angular Smart-Search ticker-to-scrip code mapper
│       ├── download.go         <-- Isolated BSE browser-footprint file downloader
│       ├── endpoints.go        <-- Registered BSE extraction strategies (15 APIs)
│       └── pipeline.go         <-- Multi-horizon interceptor & workflow pipeline
├── Dockerfile                  <-- Compact, multi-stage compilation & runtime 
├── Makefile                    <-- High-speed terminal command execution shortcuts
└── .gitignore                  <-- Keeps transient venvs and financial assets off git
```

🔥 Key Operational Features
---------------------------

*   **Parallel Chunk Streaming Downloader**: Background worker pools process tasks via specialized Go channels. Assets are streamed straight from sockets to physical disks in small blocks, keeping system RAM consumption close to 0MB even when processing hundreds of large documents.
    
*   **On-The-Fly Identity Resolution**: On BSE, string tickers ("IMFA") are dynamically mapped onto pristine 6-digit numeric internal codes (533047) using an integrated lookup bridge that intercepts the new BSE Angular search micro-service.
    
*   **Smart Local Caching**: Every background worker checks the physical disk path state before opening a socket. If a file exists locally, it skips downloading automatically (⏭️ Skipped), dramatically saving bandwidth.
    
*   **Dynamic Timeframe & Horizon Generation**: Intercepts custom APIs to smoothly step through temporal bounds sequentially (e.g., 1D, 5D, 1M, 12M chart logs, or 24-month rolling dynamic compliance lookups).
    

🛠️ Local Interface Execution Flags
-----------------------------------

The pipeline engine provides several settings via command-line arguments to customize how the application runs:

**Flag NameDefaultAllowed ValuesPurpose**\-modebothnse, bse, bothIsolates scraping target to a single exchange or runs both.-workers5Integer (e.g., 10)Adjusts the number of background threads streaming files.

🚀 Speed Shortcuts via Makefile (Dockerized)
--------------------------------------------

You do not need to have Go installed on your host machine to run this application. Everything compiles and executes inside an isolated Docker instance, routing data straight to your local hard drive via bind mounts.

### 1\. Compile the Scraper Environment

Assembles the multi-stage compiler environment and generates the statically linked execution binary layer:

Bash

`   make build   `

### 2\. Run Scraping with Defaults

Pull data for a target company across **both** exchanges using a pool of 5 default background workers:

Bash

`   make scrape ticker=IMFA   `

### 3\. Run Custom Scenarios with Advanced Arguments

Pass your custom exchange-isolation and worker execution switches cleanly inside the args="..." variable string:

Bash
`make scrape ticker=IMFA args="-mode=nse -workers=10"`
    
Bash
`make scrape ticker=TCS args="-mode=bse"`
    

📁 Storage Topography Schema
----------------------------

Downloaded files are automatically structured inside the local repository. The exchange names are cleanly prefixed onto directories to prevent folder name conflicts on disk:
data/
└── IMFA/
    ├── nse_historical-chart-data/
    ├── nse_corporate-actions/
    ├── bse_historical-chart-data/      <-- Contains (1D.json, 5D.json, etc.)
    ├── bse_bulk-block-deals/          <-- Contains (Bulk_Deals.json, Block_Deals.json)
    ├── bse_financial-results-docs/    <-- Contains physical downloaded XBRL .xml files
    └── bse_voting-results-docs/       <-- Contains physical compliance voter reports (.pdf / .html)

---

This README clearly documents the architecture and capabilities of your tool, making it easy for anyone to pick up and run your multi-exchange engine instantly!