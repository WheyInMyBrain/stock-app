# ============================================================================
# STAGE 1: COMPILE THE RUST ANALYSIS ENGINE
# ============================================================================
FROM rust:1.80-alpine AS builder
RUN apk add --no-cache musl-dev git
WORKDIR /usr/src/app

# Pre-build dummy dependencies to cache downloaded crates
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy genuine source files and compile optimized static binary targets
COPY . .
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --release

# ============================================================================
# STAGE 2: RUNTIME ENVIRONMENT
# ============================================================================
FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/analysis ./stock-analyzer

VOLUME ["/app/data"]
CMD ["./stock-analyzer"]