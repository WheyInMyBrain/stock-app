use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, USER_AGENT};
use reqwest::Client;

const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct NseClient {
    pub http_client: Client,
}

pub struct BseClient {
    pub http_client: Client,
}

unsafe extern "C" {
    fn FetchCookies(exchange: *const c_char) -> *mut c_char;
    fn FreeCString(ptr: *mut c_char);
}

fn extract_session_tokens_via_ffi(target_flag: &str) -> Result<String, String> {
    let c_flag = CString::new(target_flag).map_err(|e| e.to_string())?;
    
    unsafe {
        let raw_ptr = FetchCookies(c_flag.as_ptr());
        if raw_ptr.is_null() {
            return Err("Go FFI subsystem returned an unresolvable null pointer reference".to_string());
        }
        
        let cookie_line = CStr::from_ptr(raw_ptr).to_string_lossy().into_owned();
        FreeCString(raw_ptr);
        
        Ok(cookie_line)
    }
}

impl NseClient {
    pub async fn new() -> Result<Self, String> {
        let golden_cookies = extract_session_tokens_via_ffi("nse")?;
        
        if golden_cookies.trim().is_empty() {
            return Err("NSE requires valid session cookies, but Go FFI returned an empty string.".to_string());
        }

        let mut default_headers = HeaderMap::new();
        default_headers.insert(USER_AGENT, HeaderValue::from_static(DESKTOP_USER_AGENT));
        
        let cookie_value = HeaderValue::from_str(&golden_cookies)
            .map_err(|e| format!("Failed to serialize cookie header text: {}", e))?;
        default_headers.insert(COOKIE, cookie_value);

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .default_headers(default_headers)
            .http1_only()
            .build()
            .map_err(|e| format!("Failed to spin up reqwest framework layer: {}", e))?;

        Ok(Self { http_client: client })
    }
}

impl BseClient {
    pub async fn new() -> Result<Self, String> {
        let golden_cookies = extract_session_tokens_via_ffi("bse")?;

        let mut default_headers = HeaderMap::new();
        default_headers.insert(USER_AGENT, HeaderValue::from_static(DESKTOP_USER_AGENT));
        
        if !golden_cookies.trim().is_empty() {
            let cookie_value = HeaderValue::from_str(&golden_cookies)
                .map_err(|e| format!("Failed to serialize cookie header text: {}", e))?;
            default_headers.insert(COOKIE, cookie_value);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .default_headers(default_headers)
            .http1_only()
            .build()
            .map_err(|e| format!("Failed to spin up reqwest framework layer: {}", e))?;

        Ok(Self { http_client: client })
    }
}