// stock-app/ui/backend/src/pipeline/stock_stats.rs

use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct StockStatsCard;

impl WorkspaceModule for StockStatsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "stock_stats".to_string(),
            name: "Market Pricing & Trade Analytics".to_string(),
            description: "NSE live execution streaming node containing session boundaries, trade volume dynamics, valuation metrics, and risk limits.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let ticker_upper = ticker.to_uppercase();

        // Price Session Boundaries
        let mut open_p = "N/A".to_string();
        let mut high_p = "N/A".to_string();
        let mut low_p = "N/A".to_string();
        let mut close_p = "N/A".to_string();
        let mut prev_close = "N/A".to_string();
        let mut last_p = "N/A".to_string();
        let mut avg_p = "N/A".to_string();
        let mut net_chg = "N/A".to_string();
        let mut pct_chg = "N/A".to_string();

        // Historical Year Extremes
        let mut yr_high = "N/A".to_string();
        let mut yr_low = "N/A".to_string();
        let mut day_volatility = "N/A".to_string();
        let mut ann_volatility = "N/A".to_string();

        // Volume & Liquidity Pool Dynamics
        let mut trade_vol = "N/A".to_string();
        let mut trade_val = "N/A".to_string();
        let mut delivery_qty = "N/A".to_string();
        let mut delivery_pct = "N/A".to_string();
        let mut market_cap = "N/A".to_string();
        let mut free_float = "N/A".to_string();

        // Multiplier Valuation & Regulatory Risk Margins
        let mut sector_pe = "N/A".to_string();
        let mut symbol_pe = "N/A".to_string();
        let mut app_margin = "N/A".to_string();
        let mut var_margin = "N/A".to_string();
        let mut extreme_loss = "N/A".to_string();
        let mut impact_cost = "N/A".to_string();

        let mut last_update_time = "N/A".to_string();

        // 🚀 THE FIX: Point surgically to the target dataset using your dynamic engine selector path
        let nse_json = data.get_dataset("nse_symbol-core-data/endpoint-metadata.json");

        if let Some(eq_resp) = nse_json["equityResponse"].as_array().and_then(|a| a.first()) {
            
            if let Some(meta) = eq_resp["metaData"].as_object() {
                if let Some(v) = meta.get("open").and_then(|x| x.as_f64()) { open_p = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("dayHigh").and_then(|x| x.as_f64()) { high_p = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("dayLow").and_then(|x| x.as_f64()) { low_p = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("closePrice").and_then(|x| x.as_f64()) { close_p = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("previousClose").and_then(|x| x.as_f64()) { prev_close = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("averagePrice").and_then(|x| x.as_f64()) { avg_p = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("change").and_then(|x| x.as_f64()) { net_chg = format!("₹{:.2}", v); }
                if let Some(v) = meta.get("pChange").and_then(|x| x.as_f64()) { pct_chg = format!("{:.2}%", v); }
            }

            if let Some(trade) = eq_resp["tradeInfo"].as_object() {
                if let Some(v) = trade.get("lastPrice").and_then(|x| x.as_f64()) { last_p = format!("₹{:.2}", v); }
                if let Some(v) = trade.get("totalTradedVolume").and_then(|x| x.as_f64()) {
                    trade_vol = if v >= 100_000.0 { format!("{:.2}L", v / 100_000.0) } else { format!("{:.0}", v) };
                }
                if let Some(v) = trade.get("totalTradedValue").and_then(|x| x.as_f64()) {
                    trade_val = format!("₹{:.2}Cr", v / 10_000_000.0);
                }
                if let Some(v) = trade.get("deliveryquantity").and_then(|x| x.as_f64()) {
                    delivery_qty = if v >= 100_000.0 { format!("{:.2}L", v / 100_000.0) } else { format!("{:.0}", v) };
                }
                if let Some(v) = trade.get("deliveryToTradedQuantity").and_then(|x| x.as_f64()) { delivery_pct = format!("{:.2}%", v); }
                if let Some(v) = trade.get("totalMarketCap").and_then(|x| x.as_f64()) { market_cap = format!("₹{:.2}B", v / 1_000_000_000.0); }
                if let Some(v) = trade.get("ffmc").and_then(|x| x.as_f64()) { free_float = format!("₹{:.2}B", v / 1_000_000_000.0); }
                if let Some(v) = trade.get("impactCost").and_then(|x| x.as_f64()) { impact_cost = format!("{:.2}", v); }
            }

            if let Some(p_info) = eq_resp["priceInfo"].as_object() {
                if let Some(v) = p_info.get("yearHigh").and_then(|x| x.as_f64()) { yr_high = format!("₹{:.2}", v); }
                if let Some(v) = p_info.get("yearLow").and_then(|x| x.as_f64()) { yr_low = format!("₹{:.2}", v); }
                if let Some(v) = p_info.get("cmDailyVolatility").and_then(|x| x.as_str()) { day_volatility = format!("{}%", v); }
                if let Some(v) = p_info.get("cmAnnualVolatility").and_then(|x| x.as_str()) { ann_volatility = format!("{}%", v); }
            }

            if let Some(sec) = eq_resp["secInfo"].as_object() {
                if let Some(v) = sec.get("pdSectorPe").and_then(|x| x.as_str()) { sector_pe = v.trim().to_string(); }
                if let Some(v) = sec.get("pdSymbolPe").and_then(|x| x.as_str()) { symbol_pe = v.trim().to_string(); }
                if let Some(v) = sec.get("applicableMargin").and_then(|x| x.as_f64()) { app_margin = format!("{}%", v); }
                if let Some(v) = sec.get("varMargin").and_then(|x| x.as_f64()) { var_margin = format!("{}%", v); }
                if let Some(v) = sec.get("extremelossMargin").and_then(|x| x.as_f64()) { extreme_loss = format!("{}%", v); }
            }

            if let Some(t) = eq_resp["lastUpdateTime"].as_str() {
                last_update_time = t.to_string();
            }
        }

        // Return the full dashboard rendering card primitives tree layout payload array
        Ok(json!({
            "type": "card",
            "subtitle": format!("// EXCHANGE STREAM SESSION PROFILE: {}", ticker_upper),
            "footer": format!("NSE Feed Sync: {}", last_update_time),
            "children": [
                // CONTAINER 1: Live Price Execution Boundary 
                {
                    "type": "container",
                    "className": "w-full mb-4",
                    "children": [
                        { "type": "metric", "title": "Last Traded Price", "value": last_p },
                        { "type": "metric", "title": "Session Net Change", "value": net_chg },
                        { "type": "metric", "title": "Percentage Delta Change", "value": pct_chg },
                        { "type": "metric", "title": "VWAP (Average Price)", "value": avg_p }
                    ]
                },
                // CONTAINER 2: Session Trading Limits & Core Volume 
                {
                    "type": "container",
                    "className": "w-full mb-4",
                    "children": [
                        { "type": "metric", "title": "Opening Price", "value": open_p },
                        { "type": "metric", "title": "Intraday High", "value": high_p },
                        { "type": "metric", "title": "Intraday Low", "value": low_p },
                        { "type": "metric", "title": "Previous Close", "value": prev_close },
                        { "type": "metric", "title": "Traded Volume", "value": trade_vol },
                        { "type": "metric", "title": "Gross Traded Value", "value": trade_val }
                    ]
                },
                // CONTAINER 3: Liquidity Pools & Delivery Allocations
                {
                    "type": "container",
                    "className": "w-full mb-4",
                    "children": [
                        { "type": "metric", "title": "Delivery Quantity", "value": delivery_qty },
                        { "type": "metric", "title": "Delivery Position Ratio", "value": delivery_pct },
                        { "type": "metric", "title": "Aggregate Market Cap", "value": market_cap },
                        { "type": "metric", "title": "Free Float Market Cap", "value": free_float },
                        { "type": "metric", "title": "Current Session Close", "value": close_p } 
                    ]
                },
                // CONTAINER 4: Multiplier Valuations & Exchange Risk Guardrails
                {
                    "type": "container",
                    "className": "w-full",
                    "children": [
                        { "type": "metric", "title": "Symbol P/E", "value": symbol_pe },
                        { "type": "metric", "title": "Sector P/E", "value": sector_pe },
                        { "type": "metric", "title": "52-Week High", "value": yr_high },
                        { "type": "metric", "title": "52-Week Low", "value": yr_low },
                        { "type": "metric", "title": "Daily Volatility", "value": day_volatility },
                        { "type": "metric", "title": "Annualized Volatility", "value": ann_volatility },
                        { "type": "metric", "title": "Applicable Margin", "value": app_margin },
                        { "type": "metric", "title": "Value-at-Risk Margin", "value": var_margin },
                        { "type": "metric", "title": "Extreme Loss Margin", "value": extreme_loss },
                        { "type": "metric", "title": "Impact Cost Factor", "value": impact_cost }
                    ]
                }
            ]
        }))
    }
}