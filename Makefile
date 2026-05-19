.PHONY: docker-build scrape downloader-build downloader-run local-run help

# Default target when someone just types 'make'
help:
	@echo "========================================================================="
	@echo "🚀 DUAL-MODE FINANCIAL EXTRACTION SHUTTLE"
	@echo "========================================================================="
	@echo "🐳 DOCKERIZED TRACK (No local Go compiler required):"
	@echo "  make docker-build                         - Compile source tree inside Docker image"
	@echo "  make scrape ticker=IMFA                   - Run containerized production engine"
	@echo "  make scrape ticker=TCS args=\"-mode=nse\"   - Run container with custom parameters"
	@echo ""
	@echo "💻 LOCAL PRE-COMPILED TRACK (High Performance Binary Execution):"
	@echo "  make downloader-build                     - Compile code into optimized native binary"
	@echo "  make downloader-run ticker=IMFA           - Execute binary instantly over and over"
	@echo ""
	@echo "🧪 LOCAL ON-THE-FLY TRACK (Best for Rapid Code Dev/Testing):"
	@echo "  make local-run ticker=IMFA                - Execute directly using 'go run'"
	@echo "  make local-run ticker=TCS args=\"-workers=8\"- Run with temporary on-the-fly compilation"
	@echo "========================================================================="

# ============================================================================
# PARTITION 1: DOCKER CONTAINERIZED ENVIRONMENT
# ============================================================================

# Re-compiles your core application code inside an isolated container
docker-build:
	docker build -t stock-scraper .

# Launches the container using high-speed bind mounts to drop files directly on your host drive
# Usage: make docker-scrape ticker=IMFA args="-mode=bse"
docker-scrape:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make docker-scrape ticker=IMFA"; exit 1; fi
	docker run --rm -v "$$(pwd)/data:/root/data" stock-scraper $(ticker) $(args)


# ============================================================================
# PARTITION 2: LOCAL NATIVE BINARY ENVIRONMENT (Optimized Compilation)
# ============================================================================

# Step 1: Compile code into a standalone machine executable binary once
downloader-build:
	go build -o stock-app-scraper ./downloader/main.go
	@echo "✅ Native binary compiled successfully to root folder: ./stock-app-scraper"

# Step 2: Run that pre-compiled binary instantly with zero startup lag
# Usage: make downloader-run ticker=IMFA args="-mode=nse"
downloader-run:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make downloader-run ticker=IMFA"; exit 1; fi
	@if [ ! -f ./stock-app-scraper ]; then echo "⚠️ Local binary missing! Compiling it first..."; go build -o stock-app-scraper ./downloader/main.go; fi
	./stock-app-scraper $(args) $(ticker)


# ============================================================================
# PARTITION 3: LOCAL ON-THE-FLY TRACK (Quick Interpretation/Dev Testing)
# ============================================================================

# Bypasses Docker and manual building completely—compiles instantly in RAM and runs
# Usage: make local-run ticker=IMFA args="-mode=both -workers=10"
local-download:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make local-run ticker=IMFA"; exit 1; fi
	go run ./downloader/main.go $(args) $(ticker)