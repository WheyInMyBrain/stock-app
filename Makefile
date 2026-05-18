.PHONY: build scrape

# Re-compiles your core application code inside Docker
build:
	docker build -t stock-scraper .

# Combined automated extraction command layout
# Usage: 
#   make scrape ticker=IMFA
#   make scrape ticker=TCS args="-mode=nse -workers=10"
scrape:
	docker run --rm -v "$$(pwd)/data:/root/data" stock-scraper $(ticker) $(args)