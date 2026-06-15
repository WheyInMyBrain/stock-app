use chrono::Local;
use reqwest::header::{ACCEPT, ORIGIN, REFERER, USER_AGENT, HeaderMap, HeaderValue, HeaderName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiscEndpoint {
    InvestingHistoricalMonthly,
    GdpData,
}

impl MiscEndpoint {
    pub fn name(&self) -> &'static str {
        match self {
            MiscEndpoint::InvestingHistoricalMonthly => "investing-historical-monthly",
            MiscEndpoint::GdpData => "gdp-data",
        }
    }

    pub fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        match self {
            MiscEndpoint::InvestingHistoricalMonthly => {
                headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.5 Safari/605.1.15"));
                headers.insert(REFERER, HeaderValue::from_static("https://in.investing.com/"));
                headers.insert(ORIGIN, HeaderValue::from_static("https://in.investing.com"));
                headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
                headers.insert(
                    HeaderName::from_static("domain-id"),
                    HeaderValue::from_static("in"),
                );
            }
            MiscEndpoint::GdpData => {
                headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.5 Safari/605.1.15"));
                headers.insert(REFERER, HeaderValue::from_static("https://www.imf.org/external/datamapper/NGDP_RPCH@WEO/IND?zoom=IND&highlight=IND"));
                headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
                headers.insert(
                    HeaderName::from_static("accept-language"),
                    HeaderValue::from_static("en-US,en;q=0.9"),
                );
                headers.insert(
                    HeaderName::from_static("priority"),
                    HeaderValue::from_static("u=3, i"),
                );
                headers.insert(
                    HeaderName::from_static("sec-fetch-dest"),
                    HeaderValue::from_static("empty"),
                );
                headers.insert(
                    HeaderName::from_static("sec-fetch-mode"),
                    HeaderValue::from_static("cors"),
                );
                headers.insert(
                    HeaderName::from_static("sec-fetch-site"),
                    HeaderValue::from_static("same-origin"),
                );
            }
        }
        headers
    }

    pub fn build_url(&self) -> String {
        match self {
            MiscEndpoint::InvestingHistoricalMonthly => {
                let now = Local::now();
                let start_date = "1998-02-02";
                let end_date = now.format("%Y-%m-%d").to_string();
                format!(
                    "https://api.investing.com/api/financialdata/historical/24014?start-date={}&end-date={}&time-frame=Monthly&add-missing-rows=false",
                    start_date, end_date
                )
            }
            MiscEndpoint::GdpData => {
                "https://www.imf.org/external/datamapper/api/?meta&geoitems&indicators&datasets&values=NGDP_RPCH".to_string()
            }
        }
    }
}