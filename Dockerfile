# ============================================================================
# STAGE 1: COMPILE THE GO DOWNLOADER ENGINE
# ============================================================================
FROM golang:1.23-alpine AS go-builder
RUN apk add --no-cache git gcc musl-dev
WORKDIR /go/src/app

# Download Go dependencies using cache boundaries
COPY go.mod ./
# COPY go.sum ./ (Uncomment if you use a lockfile)
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /stock-scraper ./downloader/main.go

# ============================================================================
# STAGE 2: COMPILE THE RUST XBRL PARSER ENGINE
# ============================================================================
FROM rust:1.80-alpine AS rust-builder
RUN apk add --no-cache musl-dev git
WORKDIR /usr/src/app

# Pre-stage configuration files to cache downloaded crates
COPY ./parser/Cargo.toml ./parser/
COPY . .
WORKDIR /usr/src/app/parser

# Build high-fidelity optimized production binaries
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --release

# ============================================================================
# STAGE 3: RUNTIME LAYER - HIGHLY OPTIMIZED & DAEMONIZED PERMANENT ENGINE
# ============================================================================
FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata bash

WORKDIR /app

# Copy the statically compiled assets from their respective staging layers
COPY --from=go-builder /stock-scraper ./stock-scraper
COPY --from=rust-builder /usr/src/app/parser/target/release/parser ./stock-parser
COPY entrypoint.sh ./

RUN chmod +x ./stock-scraper ./stock-parser ./entrypoint.sh

# Establish data volume path directories so parquet and JSON maps persist on host disk
VOLUME ["/app/data"]

ENTRYPOINT ["./entrypoint.sh"]
CMD ["daemon"]