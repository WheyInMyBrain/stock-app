package scrape

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"net/http/cookiejar"
	"time"
)

const (
	UserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
	Referer   = "https://www.nseindia.com"
)

// NSEClient wraps the core client layer safely.
type NSEClient struct {
	HTTPClient *http.Client
}

// NewNSEClient initializes session context by hitting the home layout first.
func NewNSEClient() (*NSEClient, error) {
	jar, err := cookiejar.New(nil)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize cookie jar: %w", err)
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

	// Initial handshake request to catch valid session cookies
	req, err := http.NewRequest("GET", "https://www.nseindia.com", nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", UserAgent)

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("initial handshake connection dropped: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("NSE gateway rejected handshakes with status code: %d", resp.StatusCode)
	}

	return &NSEClient{HTTPClient: httpClient}, nil
}