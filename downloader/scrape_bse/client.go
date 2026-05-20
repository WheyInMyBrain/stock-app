package scrape_bse

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"net/http/cookiejar"
	"time"
)

const (
	UserAgent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
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
		Timeout: 15 * time.Second,
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{
				InsecureSkipVerify: false,
				MinVersion:         tls.VersionTLS12,
			},
			MaxIdleConns:        25,
			MaxIdleConnsPerHost: 25,
			IdleConnTimeout:     90 * time.Second,
		},
	}

	// ============================================================================
	// 🧭 STRATEGY A: THE GOOGLE REFERRAL HANDSHAKE
	// ============================================================================
	// We mimic a user searching for stock data and clicking through to BSE from Google
	req, err := http.NewRequest("GET", "https://www.bseindia.com", nil)
	if err != nil {
		return nil, err
	}

	// 🕵️‍♂️ THE SPOOF: Set a highly authentic Google search click-through footprint
	req.Header.Set("User-Agent", UserAgent)
	req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
	req.Header.Set("Accept-Language", "en-US,en;q=0.9")
	req.Header.Set("Connection", "keep-alive")
	req.Header.Set("Cache-Control", "max-age=0")
	req.Header.Set("Upgrade-Insecure-Requests", "1")
	
	// This tells their WAF that Google generated this inbound user session
	req.Header.Set("Referer", "https://www.google.com/search?q=bse+india+share+price+history")

	fmt.Println("[bse_client] 🕵️‍♂️ Executing clean Google-Referral organic entry session handshake...")
	resp, err := httpClient.Do(req)
	
	if err == nil && resp.StatusCode == http.StatusOK {
		resp.Body.Close()
		fmt.Println("🚀 [bse_client] Google handshake accepted! Session context established.")
		return &BSEClient{HTTPClient: httpClient}, nil
	}
	
	if resp != nil {
		resp.Body.Close()
	}

	// ============================================================================
	// 🛡️ STRATEGY B: THE DIRECT-TO-API FALLBACK GUARD
	// ============================================================================
	// If Strategy A gets blocked, don't let the application crash. Fall back instantly
	// to bypassing the root domain and hitting the unguarded API sub-cluster directly.
	fmt.Println("⚠️  [bse_client] Google entry handshake timed out or flagged. Deploying Direct-to-API fallback cluster...")
	
	heartbeatURL := "https://api.bseindia.com/BseIndiaAPI/api/EquityWithDetail/w?Type=EQ"
	for attempt := 1; attempt <= 2; attempt++ {
		fallbackReq, err := http.NewRequest("GET", heartbeatURL, nil)
		if err != nil {
			return nil, err
		}

		fallbackReq.Header.Set("User-Agent", UserAgent)
		fallbackReq.Header.Set("Accept", "application/json, text/plain, */*")
		fallbackReq.Header.Set("Accept-Language", "en-US,en;q=0.9")
		fallbackReq.Header.Set("Origin", Origin)
		fallbackReq.Header.Set("Referer", Referer)

		resp, err = httpClient.Do(fallbackReq)
		if err == nil && resp.StatusCode == http.StatusOK {
			resp.Body.Close()
			fmt.Println("✅ [bse_client] Fallback session cluster connected. Pipeline active.")
			return &BSEClient{HTTPClient: httpClient}, nil
		}
		
		if resp != nil {
			resp.Body.Close()
		}
		time.Sleep(2 * time.Second)
	}

	return nil, fmt.Errorf("all secure multi-exchange routing handshake scenarios exhausted")
}