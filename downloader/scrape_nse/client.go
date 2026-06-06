package scrape_nse

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"net/http/cookiejar"
	"time"
)

const (
	UserAgent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
	Referer   = "https://www.nseindia.com"
)

// NSEClient wraps the core client layer safely.
type NSEClient struct {
	HTTPClient *http.Client
}

// NewNSEClient initializes session context by hitting the home layout first with realistic headers.
func NewNSEClient() (*NSEClient, error) {
	jar, err := cookiejar.New(nil)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize cookie jar: %w", err)
	}

	httpClient := &http.Client{
		Jar:     jar,
		Timeout: 15 * time.Second, // Tightened timeout to prevent long terminal hangs
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{
				InsecureSkipVerify: false,
				MinVersion:         tls.VersionTLS12, // Enforce modern secure TLS encryption
			},
			MaxIdleConns:        25, // Increased pooling capacity to accelerate parallel worker download queues
			MaxIdleConnsPerHost: 25,
			IdleConnTimeout:     90 * time.Second,
		},
	}

	var lastErr error
	var resp *http.Response

	// 🔄 ELITE CONCURRENCY RECOVERY: Implement a 3-pass organic retry sweep.
	// If NSE drops the initial packet or flags the TLS handshake, back off and retry natively.
	for attempt := 1; attempt <= 3; attempt++ {
		req, err := http.NewRequest("GET", "https://www.nseindia.com", nil)
		if err != nil {
			return nil, err
		}

		// ============================================================================
		// 🕵️‍♂️ THE SPOOF: Complete high-fidelity browser fingerprinting headers
		// ============================================================================
		req.Header.Set("User-Agent", UserAgent)
		req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
		req.Header.Set("Accept-Language", "en-US,en;q=0.9")
		req.Header.Set("Connection", "keep-alive")
		req.Header.Set("Cache-Control", "max-age=0")
		req.Header.Set("Upgrade-Insecure-Requests", "1")
		
		// Sec headers match modern Chromium configurations natively
		req.Header.Set("Sec-Ch-Ua", `"Chromium";v="124", "Google Chrome";v="124", "Not-A.Brand";v="99"`)
		req.Header.Set("Sec-Ch-Ua-Mobile", "?0")
		req.Header.Set("Sec-Ch-Ua-Platform", `"macOS"`)
		req.Header.Set("Sec-Fetch-Dest", "document")
		req.Header.Set("Sec-Fetch-Mode", "navigate")
		req.Header.Set("Sec-Fetch-Site", "cross-site") // Flags inbound link traversal context
		req.Header.Set("Sec-Fetch-User", "?1")

		// 🧭 THE SECRET WEAPON: Tell the Akamai/Cloudflare firewall you clicked from a organic Google Search query!
		// This forces their security nodes to prioritize and approve the session link initialization.
		req.Header.Set("Referer", "https://www.google.com/search?q=nse+india+corporate+financial+results+archive")

		fmt.Printf("{NSE} 🕵️‍♂️ Initializing organic Google-Referral cookie handshake (Pass %d/3)...\n", attempt)
		resp, err = httpClient.Do(req)
		
		if err == nil {
			if resp.StatusCode == http.StatusOK {
				resp.Body.Close()
				fmt.Println("🚀 {NSE} Google-Referral handshake accepted. Secure cookie vault primed!")
				return &NSEClient{HTTPClient: httpClient}, nil
			}
			resp.Body.Close()
			lastErr = fmt.Errorf("NSE gateway rejected handshake with status code: %d", resp.StatusCode)
		} else {
			lastErr = err
		}

		// Exponential pacing buffer sleep (2s, 4s) to allow gateway rate limits to reset cleanly
		time.Sleep(time.Duration(attempt*2) * time.Second)
	}

	return nil, fmt.Errorf("NSE session pipeline initialization failed: %w", lastErr)
}