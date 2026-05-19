.PHONY: help daemon-start daemon-stop daemon-logs scrape parser docker-build local-build help

# Default target when someone just types 'make'
help:
	@echo "========================================================================="
	@echo "🪐 UNIFIED DUAL-ENGINE WORKSPACE (GO DOWNLOADER & RUST PARSER)"
	@echo "========================================================================="
	@echo "🐳 DOCKERIZED BACKGROUND DAEMON TRACK:"
	@echo "  make daemon-start                         - Build and spin up long-running daemon container"
	@echo "  make daemon-stop                          - Stop and wipe the background container"
	@echo "  make daemon-logs                          - Follow live logs streaming from daemon container"
	@echo "  make scrape ticker=IMFA                   - Fire Go scraper binary inside the live container"
	@echo "  make parser ticker=IMFA                   - Fire Rust parser binary inside the live container"
	@echo ""
	@echo "💻 LOCAL NATIVE PIPELINE COMPILED COMPONENT TRACK (Host Machine OS):"
	@echo "  make local-build                          - Compile optimized host binaries for Go & Rust"
	@echo "  make local-scrape ticker=IMFA             - Run native Go scraper binary locally"
	@echo "  make local-parse ticker=IMFA              - Run native Rust parser binary locally"
	@echo "========================================================================="

# ============================================================================
# PARTITION 1: DOCKER DAEMONIZED CONTAINER MANAGEMENT
# ============================================================================

# Builds the multi-stage image, mounts your local host /data directory, and keeps it alive
daemon-start:
	@echo "📦 Building dual-engine container ecosystem..."
	docker build -t stock-app-engine .
	@echo "🛑 Removing old container instances if present..."
	@docker rm -f market_engine 2>/dev/null || true
	@echo "🚀 Launching persistent data daemon background tracking loop..."
	docker run -d --name market_engine -v "$$(pwd)/data:/app/data" stock-app-engine daemon
	@echo "✅ Persistent background system synchronized. Test via: make scrape ticker=IMFA"

# Gracefully terminates and removes the active container workspace
daemon-stop:
	@echo "🛑 Stopping persistent background engine container..."
	docker stop market_engine || true
	docker rm market_engine || true
	@echo "✅ Infrastructure cleaned up safely."

# Streams background system tracking traces
daemon-logs:
	docker logs -f market_engine

# Executes the pre-compiled Go Downloader binary inside the live background container
scrape:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make scrape ticker=IMFA"; exit 1; fi
	docker exec -it market_engine ./stock-scraper $(ticker) $(args)

# Executes the pre-compiled Rust Parser machine binary inside the live background container
parser:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make parser ticker=IMFA"; exit 1; fi
	docker exec -it market_engine ./stock-parser $(ticker)

# Force a hard rebuild of the container layers without spinning up containers
docker-build:
	docker build --no-cache -t stock-app-engine .


# ============================================================================
# PARTITION 2: LOCAL MACHINE TRACK (Host OS Compilation)
# ============================================================================

# Compiles optimized production host binaries for both languages on your local setup
local-build:
	@echo "🦀 Compiling local Rust parser binary..."
	cd parser && cargo build --release
	@cp parser/target/release/parser ./stock-app-parser
	@echo "🦫 Compiling local Go downloader binary..."
	go build -o stock-app-scraper ./downloader/main.go
	@echo "========================================================================="
	@echo "✅ COMPILATION COMPLETE:"
	@echo "👉 Go Scraper Binary  : ./stock-app-scraper"
	@echo "👉 Rust Parser Binary : ./stock-app-parser"
	@echo "========================================================================="

# Runs your local pre-compiled Go scraper binary
local-scrape:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make local-scrape ticker=IMFA"; exit 1; fi
	@if [ ! -f ./stock-app-scraper ]; then echo "⚠️ Scraper binary missing! Building..."; go build -o stock-app-scraper ./downloader/main.go; fi
	./stock-app-scraper $(args) $(ticker)

# Runs your local pre-compiled Rust parser binary
local-parse:
	@if [ -z "$(ticker)" ]; then echo "❌ Error: 'ticker' variable is required. Example: make local-parse ticker=IMFA"; exit 1; fi
	@if [ ! -f ./stock-app-parser ]; then echo "⚠️ Parser binary missing! Building..."; cd parser && cargo build --release && cp target/release/parser ../stock-app-parser; fi
	./stock-app-parser $(ticker)