// stock-app/ui/frontend/src/ui/panels/overview.rs

use egui::{Ui, Color32, Vec2, Stroke};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator};
use backend::database::overview::OverviewMetadata;

/// Custom formatter parsing raw numbers into Indian Currency 3-2-2 digit groupings
fn format_indian_style_number(val: f64) -> String {
    if val == 0.0 { return "0".to_string(); }
    let rounded = val.round() as i64;
    let is_negative = rounded < 0;
    let clean_int = rounded.abs().to_string();

    let mut result = String::new();
    let len = clean_int.len();

    if len <= 3 {
        result.push_str(&clean_int);
    } else {
        let last_three = &clean_int[len - 3..];
        let remaining = &clean_int[..len - 3];
        
        let mut groups = Vec::new();
        let mut chars: Vec<char> = remaining.chars().collect();
        
        while !chars.is_empty() {
            let split_pos = chars.len().saturating_sub(2);
            let group: String = chars.drain(split_pos..).collect();
            groups.push(group);
        }
        groups.reverse();
        
        result.push_str(&groups.join(","));
        result.push(',');
        result.push_str(last_three);
    }

    if is_negative { format!("-{}", result) } else { result }
}

struct DetailsSubTab;
impl AbstractSubTab<OverviewMetadata> for DetailsSubTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Details" }
    
    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        ui.label(format!("ISIN: {}", meta.isin));
        ui.label(format!("NSE Code: {}", meta.nse_code));
        ui.label(format!("BSE Code: {}", meta.bse_code));
        ui.label(format!("Face Value: {}", meta.face_value));
        if !meta.nse_listing_date.is_empty() { ui.label(format!("NSE Listed Date: {}", meta.nse_listing_date)); }
        if !meta.bse_listing_date.is_empty() { ui.label(format!("BSE Listed Date: {}", meta.bse_listing_date)); }
        ui.add_space(10.0);
        ui.label(format!("Macro Category: {}", meta.macro_category));
        ui.label(format!("Sector: {}", meta.sector));
        ui.label(format!("Industry: {}", meta.industry));
        ui.add_space(10.0);
        if !meta.indexes.is_empty() {
            ui.label(egui::RichText::new("Tracked Market Indexes:").strong());
            ui.horizontal_wrapped(|ui| {
                ui.label(meta.indexes.join(", "));
            });
            ui.add_space(10.0);
        }
        if !meta.address.is_empty() { ui.label(format!("Address: {}", meta.address)); }
        if !meta.telephone.is_empty() { ui.label(format!("Telephone: {}", meta.telephone)); }
        if !meta.fax.is_empty() { ui.label(format!("Fax: {}", meta.fax)); }
        if !meta.email.is_empty() { ui.label(format!("Email: {}", meta.email)); }
        if !meta.website.is_empty() { ui.label(format!("Website: {}", meta.website)); }
    }
}

struct ShareholdersMatrixSubTab;
impl AbstractSubTab<OverviewMetadata> for ShareholdersMatrixSubTab {
    fn id(&self) -> usize { 3 }
    fn label(&self) -> &'static str { "Shareholders Structure" }

    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        if meta.available_reporting_dates.is_empty() {
            ui.weak("No regulatory shareholding data logs cached for this ticker.");
            return;
        }

        // Get unique rows to keep everything structured properly
        let unique_macro_labels: Vec<&str> = meta
            .macro_allocations
            .iter()
            .map(|r| r.category_label.as_str())
            .collect::<std::collections::BTreeSet<&str>>()
            .into_iter()
            .collect();

        let unique_whale_names: Vec<(&str, &str)> = meta
            .hni_whales
            .iter()
            .map(|r| (r.investor_name.as_str(), r.entity_classification.as_str()))
            .collect::<std::collections::BTreeSet<(&str, &str)>>()
            .into_iter()
            .collect();

        egui::ScrollArea::vertical()
            .id_source("shareholders_dead_simple_vertical_scroll")
            .show(ui, |ui| {

                let panel_max_safe_width = ui.available_width();

                ui.label(egui::RichText::new("CAP-TABLE MACRO STRUCTURE ALLOCATION").strong().color(Color32::from_rgb(100, 180, 240)));
                ui.add_space(6.0);

                egui::ScrollArea::horizontal()
                    .id_source("macro_horizontal_containment_shield")
                    .max_width(panel_max_safe_width)
                    .show(ui, |ui| {
                        egui::Grid::new("macro_allocations_simple_grid")
                            .striped(true)
                            .spacing(Vec2::new(32.0, 12.0))
                            .show(ui, |ui| {
                                // Headers
                                ui.label(egui::RichText::new("SEGMENT PARTICULARS").strong());
                                for quarter_date in &meta.available_reporting_dates {
                                    ui.label(egui::RichText::new(quarter_date).strong().color(Color32::WHITE));
                                }
                                ui.end_row();

                                // Data rows
                                for label in &unique_macro_labels {
                                    ui.label(egui::RichText::new(*label).strong());
                                    
                                    for quarter_date in &meta.available_reporting_dates {
                                        let cell_match = meta.macro_allocations.iter().find(|r| r.date == *quarter_date && r.category_label == *label);
                                        ui.vertical(|ui| {
                                            if let Some(row) = cell_match {
                                                ui.label(egui::RichText::new(format!("{:.2}%", row.stake_percentage)).color(Color32::from_rgb(100, 180, 240)).strong());
                                                ui.small(format_indian_style_number(row.share_count));
                                            } else {
                                                ui.weak("0.00%");
                                                ui.small("0");
                                            }
                                        });
                                    }
                                    ui.end_row();
                                }
                            });
                    });

                ui.add_space(28.0);

                ui.label(egui::RichText::new("STRATEGIC HNI WHALES DISCLOSURE MATRIX").strong().color(Color32::from_rgb(100, 240, 140)));
                ui.add_space(6.0);

                if unique_whale_names.is_empty() {
                    ui.weak("↳ No individual corporate bodies or whales holding >= 1% registered.");
                } else {
                    egui::ScrollArea::horizontal()
                        .id_source("whale_horizontal_containment_shield")
                        .max_width(panel_max_safe_width)
                        .show(ui, |ui| {
                            egui::Grid::new("whale_allocations_simple_grid")
                                .striped(true)
                                .spacing(Vec2::new(32.0, 12.0))
                                .show(ui, |ui| {
                                    // Headers
                                    ui.label(egui::RichText::new("INVESTOR LEGAL IDENTITY [CLASSIFICATION]").strong());
                                    for quarter_date in &meta.available_reporting_dates {
                                        ui.label(egui::RichText::new(quarter_date).strong().color(Color32::WHITE));
                                    }
                                    ui.end_row();

                                    // Data rows
                                    for (name, class_tag) in &unique_whale_names {
                                        ui.label(format!("{} [{}]", name, class_tag));
                                        
                                        for quarter_date in &meta.available_reporting_dates {
                                            let whale_match = meta.hni_whales.iter().find(|r| r.date == *quarter_date && r.investor_name == *name);
                                            ui.vertical(|ui| {
                                                if let Some(row) = whale_match {
                                                    ui.label(egui::RichText::new(format!("{:.2}%", row.stake_percentage)).color(Color32::from_rgb(100, 240, 140)).strong());
                                                    ui.small(format_indian_style_number(row.share_count));
                                                } else {
                                                    ui.weak("0.00%");
                                                    ui.small("0");
                                                }
                                            });
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                }

                ui.add_space(28.0);

                ui.label(egui::RichText::new("LATEST REVEALED SIGNIFICANT BENEFICIAL OWNERSHIP (SBO DIRECTORY)").strong().color(Color32::from_rgb(240, 180, 100)));
                ui.add_space(6.0);

                if meta.sbo_registry.is_empty() {
                    ui.weak("↳ No complex indirect SBO human governance command tracks reported inside this asset catalog.");
                } else {
                    let mut active_sbo_quarter = String::new();
                    let mut filtered_sbos = Vec::new();

                    for date in &meta.available_reporting_dates {
                        let matches: Vec<_> = meta.sbo_registry.iter().filter(|r| r.date == *date).collect();
                        if !matches.is_empty() {
                            active_sbo_quarter = date.clone();
                            filtered_sbos = matches;
                            break;
                        }
                    }

                    if filtered_sbos.is_empty() {
                        ui.weak("↳ No active SBO layers reported in recent quarters.");
                    } else {
                        ui.label(egui::RichText::new(format!("Showing latest available data from: {}", active_sbo_quarter)).italics().color(Color32::GRAY));
                        ui.add_space(8.0);

                        // Render as a flat vertical list that naturally wraps and stays inside your side-panel boundaries
                        ui.vertical(|ui| {
                            for row in filtered_sbos {
                                let item_frame = egui::Frame::none()
                                    .fill(Color32::from_rgb(22, 22, 22))
                                    .inner_margin(egui::Margin::same(12.0))
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(40, 45, 50)));

                                item_frame.show(ui, |ui| {
                                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.horizontal(|ui| {
                                            ui.weak("Ultimate Human SBO: ");
                                            ui.label(egui::RichText::new(&row.human_sbo_name).strong().color(Color32::WHITE));
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.weak("Registered Proxy Shell / Trust: ");
                                            ui.label(egui::RichText::new(&row.proxy_registered_owner).color(Color32::from_rgb(240, 180, 100)));
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.weak("Origin & Acquisition Date: ");
                                            ui.label(format!("{} ({})", row.nationality, row.acquisition_date));
                                        });
                                    });
                                });
                                ui.add_space(8.0);
                            }
                        });
                    }
                }
            });
    }

    fn render_bottom(&self, _ui: &mut Ui, _meta: &OverviewMetadata) {}
}

struct BoardSubTab;
impl AbstractSubTab<OverviewMetadata> for BoardSubTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Board of Directors" }
    
    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        ui.label(egui::RichText::new("BOARD OF DIRECTORS").strong());
        ui.add_space(10.0);
        for dir in &meta.directors {
            ui.label(format!("• {} ({})", dir.name, dir.designation));
        }
    }
}

struct InvestorComplaintsSubTab;
impl AbstractSubTab<OverviewMetadata> for InvestorComplaintsSubTab {
    fn id(&self) -> usize { 4 } // Assign next sequential index slot
    fn label(&self) -> &'static str { "Investor Complaints" }

    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        if meta.investor_complaints.is_empty() {
            ui.weak("No regulatory investor complaint data logs filed for this ticker.");
            return;
        }

        ui.label(egui::RichText::new("HISTORICAL INVESTOR COMPLAINTS MATRIX").strong().color(Color32::from_rgb(240, 110, 110)));
        ui.add_space(8.0);

        // Keep it safe inside a single horizontal scroll view wrapper container
        egui::ScrollArea::horizontal()
            .id_source("investor_complaints_horizontal_scroller")
            .max_width(ui.available_width())
            .show(ui, |ui| {
                egui::Grid::new("complaints_timeline_grid")
                    .striped(true)
                    .spacing(Vec2::new(32.0, 14.0))
                    .show(ui, |ui| {
                        
                        // --- ROW 1: HEADER QUARTER DATE LABELS ---
                        ui.label(egui::RichText::new("METRIC PARAMETER TRACK").strong().heading());
                        for row in &meta.investor_complaints {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&row.date).strong().color(Color32::WHITE));
                            });
                        }
                        ui.end_row();

                        // --- ROW 2: PENDING AT BEGINNING OF PERIOD ---
                        ui.label("Complaints Pending (Beginning)");
                        for row in &meta.investor_complaints {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{:.0}", row.complaints_beginning));
                            });
                        }
                        ui.end_row();

                        // --- ROW 3: RECEIVED DURING PERIOD ---
                        ui.label("Complaints Received");
                        for row in &meta.investor_complaints {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.0}", row.complaints_received)).color(Color32::from_rgb(255, 165, 0)));
                            });
                        }
                        ui.end_row();

                        // --- ROW 4: DISPOSED OF DURING PERIOD ---
                        ui.label("Complaints Disposed / Resolved");
                        for row in &meta.investor_complaints {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{:.0}", row.complaints_disposed)).color(Color32::from_rgb(100, 240, 140)));
                            });
                        }
                        ui.end_row();

                        // --- ROW 5: UNRESOLVED END OF PERIOD BACKLOG ---
                        ui.label(egui::RichText::new("Net Unresolved Backlog").strong());
                        for row in &meta.investor_complaints {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let color = if row.complaints_unresolved > 0.0 { Color32::LIGHT_RED } else { Color32::GRAY };
                                ui.label(egui::RichText::new(format!("{:.0}", row.complaints_unresolved)).color(color).strong());
                            });
                        }
                        ui.end_row();
                    });
            });
    }

    fn render_bottom(&self, _ui: &mut Ui, _meta: &OverviewMetadata) {}
}

struct PeerComparisonSubTab;
impl AbstractSubTab<OverviewMetadata> for PeerComparisonSubTab {
    fn id(&self) -> usize { 5 }
    fn label(&self) -> &'static str { "Peer Comparison Matrix" }

    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        if meta.peer_comparisons.is_empty() {
            ui.weak("No peer comparison datasets found for this ticker.");
            return;
        }

        let cat_state_id = ui.make_persistent_id("peer_comparison_category_token");
        let mut selected_category = meta.peer_comparisons.first().unwrap().category_name.clone();
        ui.data_mut(|d| {
            if let Some(cached) = d.get_persisted::<String>(cat_state_id) {
                if meta.peer_comparisons.iter().any(|c| c.category_name == cached) {
                    selected_category = cached;
                }
            }
        });

        let active_cat = meta.peer_comparisons.iter().find(|c| c.category_name == selected_category).unwrap();

        let date_state_id = ui.make_persistent_id("peer_comparison_date_token");
        let mut selected_date = active_cat.available_dates.first().cloned().unwrap_or_default();
        ui.data_mut(|d| {
            if let Some(cached) = d.get_persisted::<String>(date_state_id) {
                if active_cat.available_dates.contains(&cached) {
                    selected_date = cached;
                }
            }
        });

        // Horizontal period toggles
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Reporting Period:").strong().color(Color32::from_rgb(100, 180, 240)));
            for d in &active_cat.available_dates {
                if ui.selectable_label(*d == selected_date, d).clicked() {
                    selected_date = d.clone();
                    ui.data_mut(|d_store| d_store.insert_persisted(date_state_id, selected_date.clone()));
                }
            }
        });
        ui.add_space(12.0);

        if let Some(records) = active_cat.date_matrices.get(&selected_date) {
            let max_safe_panel_width = ui.available_width();

            // Contain the grid cleanly inside a width-bounded horizontal scroller
            egui::ScrollArea::horizontal()
                .id_source("peer_matrix_horizontal_scroller_guard")
                .max_width(max_safe_panel_width)
                .show(ui, |ui| {
                    egui::Grid::new("peer_comparison_table")
                        .striped(true)
                        .spacing(Vec2::new(28.0, 12.0))
                        .show(ui, |ui| {
                            let headers = ["Symbol", "LTP", "P. Change", "Market Cap", "P/E", "EPS", "PAT", "Total Income", "Promoter Hold", "Debt/Eq"];
                            for h in headers {
                                ui.label(egui::RichText::new(h).strong().color(Color32::WHITE));
                            }
                            ui.end_row();

                            for rec in records {
                                ui.label(egui::RichText::new(&rec.symbol).strong().color(Color32::from_rgb(100, 180, 240)));
                                ui.label(format_indian_style_number(rec.ltp));
                                
                                let color = if rec.p_change < 0.0 { Color32::LIGHT_RED } else if rec.p_change > 0.0 { Color32::from_rgb(100, 240, 140) } else { Color32::GRAY };
                                ui.label(egui::RichText::new(format!("{:.2}%", rec.p_change)).color(color));
                                
                                ui.label(format_indian_style_number(rec.market_cap));
                                ui.label(format!("{:.2}", rec.pe));
                                ui.label(format!("{:.2}", rec.eps));
                                ui.label(format_indian_style_number(rec.pat));
                                ui.label(format_indian_style_number(rec.total_income));
                                ui.label(format!("{:.2}%", rec.promoter_holding));
                                ui.label(&rec.debt_eq_ratio);
                                ui.end_row();
                            }
                        });
                });
        }
    }

    fn render_bottom(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        if meta.peer_comparisons.is_empty() { return; }

        let cat_state_id = ui.make_persistent_id("peer_comparison_category_token");
        let mut selected_category = meta.peer_comparisons.first().unwrap().category_name.clone();
        ui.data_mut(|d| {
            if let Some(cached) = d.get_persisted::<String>(cat_state_id) {
                if meta.peer_comparisons.iter().any(|c| c.category_name == cached) {
                    selected_category = cached;
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("PEER MATRIX DOMAINS:").strong());
            ui.add_space(10.0);
            
            egui::ScrollArea::horizontal()
                .id_source("peer_comparison_bottom_scroll")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                    for cat in &meta.peer_comparisons {
                        let is_selected = cat.category_name == selected_category;
                        let btn_text = egui::RichText::new(&cat.category_name).strong();
                        
                        let button_widget = egui::Button::new(btn_text)
                            .min_size(Vec2::new(84.0, 36.0))
                            .stroke(if is_selected {
                                Stroke::new(2.0, Color32::from_rgb(100, 180, 240)) 
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
                            });

                        if ui.add(button_widget).clicked() {
                            selected_category = cat.category_name.clone();
                            ui.data_mut(|d| d.insert_persisted(cat_state_id, selected_category.clone()));
                        }
                    }
                });
        });
    }
}

pub fn draw_overview_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_overview_data(active_ticker);

    let tabs: &[&dyn AbstractSubTab<OverviewMetadata>] = &[
        &DetailsSubTab,
        &ShareholdersMatrixSubTab,
        &BoardSubTab,
        &InvestorComplaintsSubTab,
        &PeerComparisonSubTab,
    ];

    draw_nav_canvas_orchestrator(
        ui,
        active_ticker,
        "overview_metadata",        
        "OVERVIEW",                 
        "overview_active_sub_tab",  
        tabs,
    );
}