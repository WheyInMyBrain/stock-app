# ============================================================================
# STAGE 1: COMPILE THE GO APPLICATION ENGINE
# ============================================================================
FROM golang:1.23-alpine AS builder

# Install system utilities needed for potential cross-compilation boundaries
RUN apk add --no-cache git gcc musl-dev

# Set the working internal staging container directory
WORKDIR /app

# Copy the dependency tracking manifests first to maximize layer caching
COPY go.mod ./
# If you have a go.sum file, uncomment the line below:
# COPY go.sum ./
RUN go mod download

# Copy the entire source code tree into the builder environment
COPY . .

# Compile the downloader module binary as a statically linked standalone artifact
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /stock-scraper ./downloader/main.go

# ============================================================================
# STAGE 2: HIGHLY COMPACT PRODUCTION RUNTIME ENVIRONMENT
# ============================================================================
FROM alpine:3.19

# Install ca-certificates so HTTPS connections to NSE/BSE API subdomains work perfectly
RUN apk add --no-cache ca-certificates tzdata

WORKDIR /root/

# Copy the compiled binary out from our staging builder container layer
COPY --from=builder /stock-scraper .

# Establish a default fallback execution instruction loop command
ENTRYPOINT ["./stock-scraper"]
CMD ["--help"]