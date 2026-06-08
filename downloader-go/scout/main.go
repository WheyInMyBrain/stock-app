package main

/*
#include <stdlib.h>
*/
import "C"

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"net/http/cookiejar"
	"net/url"
	"time"
	"unsafe"
)

const UserAgent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"

func main() {}

//export FetchCookies
func FetchCookies(exchange *C.char) *C.char {
	target := C.GoString(exchange)
	jar, _ := cookiejar.New(nil)
	httpClient := &http.Client{
		Jar:     jar,
		Timeout: 15 * time.Second,
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{MinVersion: tls.VersionTLS12},
		},
	}

	if target == "nse" {
		req, _ := http.NewRequest("GET", "https://www.nseindia.com", nil)
		req.Header.Set("User-Agent", UserAgent)
		req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
		req.Header.Set("Accept-Language", "en-US,en;q=0.9")
		req.Header.Set("Connection", "keep-alive")
		req.Header.Set("Cache-Control", "max-age=0")
		req.Header.Set("Upgrade-Insecure-Requests", "1")
		req.Header.Set("Sec-Ch-Ua", `"Chromium";v="124", "Google Chrome";v="124", "Not-A.Brand";v="99"`)
		req.Header.Set("Sec-Ch-Ua-Mobile", "?0")
		req.Header.Set("Sec-Ch-Ua-Platform", `"macOS"`)
		req.Header.Set("Sec-Fetch-Dest", "document")
		req.Header.Set("Sec-Fetch-Mode", "navigate")
		req.Header.Set("Sec-Fetch-Site", "cross-site")
		req.Header.Set("Sec-Fetch-User", "?1")
		req.Header.Set("Referer", "https://www.google.com/search?q=nse+india+corporate+financial+results+archive")

		resp, err := httpClient.Do(req)
		if err == nil && resp.StatusCode == http.StatusOK {
			resp.Body.Close()
			return C.CString(getCookieString(jar, req.URL))
		}
		if resp != nil {
			resp.Body.Close()
		}
		return C.CString("")

	} else if target == "bse" {
		req, _ := http.NewRequest("GET", "https://www.bseindia.com", nil)
		req.Header.Set("User-Agent", UserAgent)
		req.Header.Set("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
		req.Header.Set("Accept-Language", "en-US,en;q=0.9")
		req.Header.Set("Connection", "keep-alive")
		req.Header.Set("Cache-Control", "max-age=0")
		req.Header.Set("Upgrade-Insecure-Requests", "1")
		req.Header.Set("Referer", "https://www.google.com/search?q=bse+india+share+price+history")

		resp, err := httpClient.Do(req)
		if err == nil && resp.StatusCode == http.StatusOK {
			resp.Body.Close()
			return C.CString(getCookieString(jar, req.URL))
		}
		if resp != nil {
			resp.Body.Close()
		}

		heartbeatURL := "https://api.bseindia.com/BseIndiaAPI/api/EquityWithDetail/w?Type=EQ"
		fallbackReq, _ := http.NewRequest("GET", heartbeatURL, nil)
		fallbackReq.Header.Set("User-Agent", UserAgent)
		fallbackReq.Header.Set("Accept", "application/json, text/plain, */*")
		fallbackReq.Header.Set("Accept-Language", "en-US,en;q=0.9")
		fallbackReq.Header.Set("Origin", "https://www.bseindia.com")
		fallbackReq.Header.Set("Referer", "https://www.bseindia.com/")

		resp, err = httpClient.Do(fallbackReq)
		if err == nil && resp.StatusCode == http.StatusOK {
			resp.Body.Close()
			return C.CString(getCookieString(jar, fallbackReq.URL))
		}
		if resp != nil {
			resp.Body.Close()
		}
		return C.CString("")
	}

	return C.CString("")
}

func getCookieString(jar *cookiejar.Jar, u *url.URL) string {
	cookies := jar.Cookies(u)
	cookieString := ""
	for i, c := range cookies {
		if i > 0 {
			cookieString += "; "
		}
		cookieString += fmt.Sprintf("%s=%s", c.Name, c.Value)
	}
	return cookieString
}

//export FreeCString
func FreeCString(ptr *C.char) {
	C.free(unsafe.Pointer(ptr))
}