package scrape_bse

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"net/http/cookiejar"
	"time"
)

const (
	UserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
	Origin    = "https://www.bseindia.com"
	Referer   = "https://www.bseindia.com/"
)

// BSEClient wraps the underlying HTTP client session context for Bombay Stock Exchange operations.
type BSEClient struct {
	HTTPClient *http.Client
}

// NewBSEClient configures the session layer, cookie management, and standard transport rules.
func NewBSEClient() (*BSEClient, error) {
	jar, err := cookiejar.New(nil)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize bse cookie jar: %w", err)
	}

	httpClient := &http.Client{
		Jar:     jar,
		Timeout: 20 * time.Second,
		Transport: &http.Transport{
			TLSClientConfig:     &tls.Config{InsecureSkipVerify: false},
			MaxIdleConns:        10,
			MaxIdleConnsPerHost: 10,
			IdleConnTimeout:     60 * time.Second,
		},
	}

	// Initial heartbeat hit to the main domain to acquire standard initialization cookies if required
	req, err := http.NewRequest("GET", "https://www.bseindia.com", nil)
	if err != nil {
		return nil, err
	}
	
	req.Header.Set("User-Agent", UserAgent)
	req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
	req.Header.Set("Accept-Language", "en-US,en;q=0.9")

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("initial BSE domain handshake connection failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("BSE core landing portal rejected initial handshake with code: %d", resp.StatusCode)
	}

	return &BSEClient{HTTPClient: httpClient}, nil
}