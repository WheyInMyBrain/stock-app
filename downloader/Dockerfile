# ============================================================================
# STAGE 1: COMPILE THE GO DOWNLOADER ENGINE
# ============================================================================
FROM golang:1.23-alpine AS builder
RUN apk add --no-cache git gcc musl-dev
WORKDIR /go/src/app

# Leverage caching layers for module downloads
COPY go.mod ./
# COPY go.sum ./ 
RUN go mod download

# Copy source and strip symbol tables for a tiny binary footprint
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /stock-scraper main.go

# ============================================================================
# STAGE 2: RUNTIME ENVIROMENT
# ============================================================================
FROM alpine:3.19
RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app
COPY --from=builder /stock-scraper .

VOLUME ["/app/data"]
CMD ["./stock-scraper"]