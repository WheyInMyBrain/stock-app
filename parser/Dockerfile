# ============================================================================
# STAGE 1: COMPILE THE RUST XBRL PARSER ENGINE
# ============================================================================
FROM rust:1.80-alpine AS builder
RUN apk add --no-cache musl-dev git
WORKDIR /usr/src/app

# Cache dependencies independently
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy real source code and compile optimized production runtime binary
COPY . .
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --release

# ============================================================================
# STAGE 2: RUNTIME ENVIRONMENT
# ============================================================================
FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/parser ./stock-parser

VOLUME ["/app/data"]
CMD ["./stock-parser"]