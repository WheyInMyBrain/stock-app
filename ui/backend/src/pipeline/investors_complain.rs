use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct InvestorsComplainCard;

impl WorkspaceModule for InvestorsComplainCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "investors_complain".to_string(),
            name: "Investor Complaints & Governance Tracker".to_string(),
            description: "Compiles historic regulatory filings of corporate complaints, unresolved burdens, and resolution efficiencies with an automated governance score.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let nse_json = data.get_dataset("nse_investor-complaints/endpoint-metadata.json");
        
        let mut filings = Vec::new();
        if let Some(arr) = nse_json.as_array() {
            filings = arr.clone();
        } else if let Some(arr) = nse_json["data"].as_array() {
            filings = arr.clone();
        }

        let mut total_beg = 0.0;
        let mut total_recv = 0.0;
        let mut total_disp = 0.0;
        let mut total_unres = 0.0;

        let mut max_backlog = 0.0;
        let mut max_backlog_date = "N/A".to_string();

        let mut timeline_points = Vec::new();

        for filing in &filings {
            let date_str = filing.get("date").and_then(|v| v.as_str()).unwrap_or("N/A");
            let beg = filing.get("complBeg").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let recv = filing.get("complRecv").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let disp = filing.get("complDisp").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let unres = filing.get("complUnres").and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            total_beg += beg;
            total_recv += recv;
            total_disp += disp;
            total_unres += unres;

            let current_backlog = beg + unres;
            if current_backlog > max_backlog {
                max_backlog = current_backlog;
                max_backlog_date = date_str.to_string();
            }

            timeline_points.push((date_str.to_string(), recv, disp, unres, beg));
        }

        timeline_points.reverse();

        let mut max_recv = 0.0;
        let mut min_recv = f64::MAX;
        let mut max_date = "N/A".to_string();
        let mut min_date = "N/A".to_string();

        for (date, recv, _, _, _) in &timeline_points {
            if *recv > max_recv {
                max_recv = *recv;
                max_date = date.clone();
            }
            if *recv < min_recv {
                min_recv = *recv;
                min_date = date.clone();
            }
        }
        if min_recv == f64::MAX { min_recv = 0.0; }

        let total_points = timeline_points.len();
        let mut trend_slope = 0.0;
        if total_points > 1 {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut sum_xy = 0.0;
            let mut sum_xx = 0.0;
            for idx in 0..total_points {
                let x = idx as f64;
                let y = timeline_points[idx].1;
                sum_x += x;
                sum_y += y;
                sum_xy += x * y;
                sum_xx += x * x;
            }
            let n = total_points as f64;
            let denominator = n * sum_xx - sum_x * sum_x;
            if denominator != 0.0 {
                trend_slope = (n * sum_xy - sum_x * sum_y) / denominator;
            }
        }

        let total_workload = total_beg + total_recv;
        let mut governance_health_score = if total_workload > 0.0 {
            let resolution_efficiency = total_disp / total_workload;
            let mut score = resolution_efficiency * 60.0;

            if total_unres > 0.0 {
                let unresolved_penalty = (total_unres / total_workload) * 25.0;
                let backlog_weight_multiplier = 1.0 + (total_unres / 10.0);
                score -= unresolved_penalty * backlog_weight_multiplier;
            }
            score
        } else {
            60.0
        };

        if trend_slope < 0.0 {
            let trend_bonus = (trend_slope.abs() * 15.0).min(40.0);
            governance_health_score += trend_bonus;
        } else if trend_slope > 0.0 {
            let trend_malus = (trend_slope * 15.0).min(30.0);
            governance_health_score -= trend_malus;
        }

        if total_recv > 50.0 {
            let volume_penalty = ((total_recv - 50.0) / 50.0).min(10.0);
            governance_health_score -= volume_penalty;
        }

        if governance_health_score < 0.0 { governance_health_score = 0.0; }
        if governance_health_score > 100.0 { governance_health_score = 100.0; }
        if filings.is_empty() { governance_health_score = 100.0; }

        let status_label = if governance_health_score >= 90.0 {
            "OPTIMAL // PROGRESSIVE DE-ESCALATION"
        } else if governance_health_score >= 75.0 {
            "STABLE // CONVOLUTED RESOLUTIONS"
        } else {
            "REGULATORY DEFICIT // INCOMING VELOCITY EXCEEDS OUTPUT"
        };

        let mut card_children = Vec::new();

        // 🚀 METRIC STATS BLOCK 
        let mut stats_grid = vec![
            json!({
                "type": "container",
                "className": "flex flex-col gap-0.5",
                "children": [
                    { "type": "text", "className": "text-[10px] tracking-wider uppercase font-bold opacity-40 font-mono", "value": "Maximum Complaints" },
                    { "type": "text", "className": "text-lg font-bold font-mono tracking-tight", "value": format!("{:.0}", max_recv) },
                    { "type": "text", "className": "text-[9px] opacity-50 font-mono lowercase", "value": format!("recorded at: {}", max_date) }
                ]
            }),
            json!({
                "type": "container",
                "className": "flex flex-col gap-0.5 text-right items-end",
                "children": [
                    { "type": "text", "className": "text-[10px] tracking-wider uppercase font-bold opacity-40 font-mono", "value": "Minimum Complaints" },
                    { "type": "text", "className": "text-lg font-bold font-mono tracking-tight", "value": format!("{:.0}", min_recv) },
                    { "type": "text", "className": "text-[9px] opacity-50 font-mono lowercase", "value": format!("recorded at: {}", min_date) }
                ]
            })
        ];

        if max_backlog > 0.0 {
            stats_grid.push(json!({
                "type": "container",
                "className": "flex flex-col gap-0.5 mt-2 pt-2 border-t border-dashed col-span-2 text-left w-full",
                "style": { "gridColumn": "span 2" },
                "children": [
                    { "type": "text", "className": "text-[10px] tracking-wider uppercase font-bold opacity-40 font-mono", "value": "Peak Historic Backlog Burden" },
                    { "type": "text", "className": "text-sm font-bold font-mono text-amber-500 tracking-tight", "value": format!("{:.0}", max_backlog) },
                    { "type": "text", "className": "text-[9px] opacity-50 font-mono lowercase", "value": format!("unresolved bottleneck window: {}", max_backlog_date) }
                ]
            }));
        }

        card_children.push(json!({
            "type": "container",
            "className": "w-full grid grid-cols-2 gap-4 mb-4 pb-2",
            "style": { "gridTemplateColumns": "1fr 1fr" },
            "children": stats_grid
        }));

        let mut vector_elements = Vec::new();

        if !timeline_points.is_empty() {
            let x_start = 40.0;
            let x_end = 490.0;
            let y_baseline = 140.0;
            let y_top = 20.0;

            let ceil_max = if max_recv > 0.0 { (max_recv / 5.0).ceil() * 5.0 } else { 10.0 };
            let y_scale = (y_baseline - y_top) / ceil_max;

            let x_step = if total_points > 1 {
                (x_end - x_start) / (total_points - 1) as f64
            } else {
                0.0
            };

            let mut segments = Vec::new();

            for idx in 0..total_points {
                let (date, recv, disp, unres, beg) = &timeline_points[idx];
                let x = x_start + (idx as f64 * x_step);
                let y = y_baseline - (recv * y_scale);

                let factor = if ceil_max > 0.0 { recv / ceil_max } else { 0.0 };
                
                let (r, g, b) = if factor < 0.5 {
                    let segment_factor = factor * 2.0;
                    (
                        (0.0 + (245.0 - 0.0) * segment_factor) as u8,
                        (185.0 + (158.0 - 185.0) * segment_factor) as u8,
                        (129.0 + (11.0 - 129.0) * segment_factor) as u8,
                    )
                } else {
                    let segment_factor = (factor - 0.5) * 2.0;
                    (
                        (245.0 + (239.0 - 245.0) * segment_factor) as u8,
                        (158.0 + (68.0 - 158.0) * segment_factor) as u8,
                        (11.0 + (68.0 - 11.0) * segment_factor) as u8,
                    )
                };
                
                let hex_color = format!("#{:02x}{:02x}{:02x}", r, g, b);

                if idx > 0 {
                    let prev_idx = idx - 1;
                    let (_, prev_recv, _, _, _) = &timeline_points[prev_idx];
                    let prev_x = x_start + (prev_idx as f64 * x_step);
                    let prev_y = y_baseline - (prev_recv * y_scale);

                    segments.push(json!({
                        "type": "vector_path",
                        "d": format!("M {} {} L {} {}", prev_x, prev_y, x, y),
                        "stroke": hex_color,
                        "stroke_width": 2,
                        "fill": "none"
                    }));
                }

                let tooltip_text = format!(
                    "Period: {}\nBrought Forward: {}\nReceived: {}\nResolved: {}\nUnresolved Roll-Over: {}", 
                    date, beg, recv, disp, unres
                );

                vector_elements.push(json!({
                    "type": "vector_rect",
                    "x": x - 3.0,
                    "y": y - 3.0,
                    "width": 6,
                    "height": 6,
                    "fill": hex_color,
                    "tooltip": tooltip_text,
                    "className": "cursor-pointer transition-all hover:scale-150"
                }));

                if idx == 0 || idx == total_points - 1 || total_points <= 5 {
                    vector_elements.push(json!({
                        "type": "text",
                        "style": { "x": x - 20.0, "y": 155, "transform": "" },
                        "className": "opacity-40 font-mono",
                        "value": date
                    }));
                }
            }

            for segment in segments {
                vector_elements.insert(0, segment);
            }
        }

        card_children.push(json!({
            "type": "container",
            "className": "w-full flex flex-col flex-1",
            "children": [
                {
                    "type": "vector_canvas",
                    "children": vector_elements
                }
            ]
        }));

        Ok(json!({
            "type": "card",
            "title": format!("{:.1}% GOVERNANCE HEALTH", governance_health_score),
            "subtitle": format!("// {} // TARGET SYMBOL NODE: {}", status_label, ticker.to_uppercase()),
            "footer": format!("Aggregated Audit: Received [{}], Disposed [{}], Outstanding Unresolved [{}]", total_recv, total_disp, total_unres),
            "children": card_children
        }))
    }
}