use egui::{Ui, Color32, Frame, Margin, Stroke, Pos2, Vec2, Rect};
use std::collections::{HashSet, HashMap, BTreeSet};

pub trait AbstractSubTab<T> {
    fn id(&self) -> usize;
    fn label(&self) -> &'static str;
    fn render_main(&self, ui: &mut Ui, data: &T);
    fn render_bottom(&self, _ui: &mut Ui, _data: &T) {}
}

pub fn draw_three_zone_canvas<M, B, S>(
    ui: &mut Ui,
    render_main: M,
    render_bottom: B,
    render_side: S,
) where
    M: FnOnce(&mut Ui),
    B: FnOnce(&mut Ui),
    S: FnOnce(&mut Ui),
{
    let screen_width_points = ui.ctx().input(|i| i.screen_rect.width());
    let current_zoom = ui.ctx().zoom_factor();
    let unscaled_raw_width = screen_width_points * current_zoom;

    let target_zoom = (unscaled_raw_width / 1366.0).clamp(0.75, 1.20);
    if (current_zoom - target_zoom).abs() > 0.01 {
        ui.ctx().set_zoom_factor(target_zoom);
    }

    let total_width = ui.available_width();
    let total_height = ui.available_height();
    let spacing = ui.spacing().item_spacing;

    let side_width = (total_width * 0.18).clamp(160.0, 260.0);
    let left_width = total_width - side_width - spacing.x;

    let ideal_main_height = left_width * (9.0 / 16.0);
    let max_allowed_main_height = total_height - 130.0 - spacing.y; 
    let main_height = ideal_main_height.min(max_allowed_main_height).max(160.0);
    let bottom_height = total_height - main_height - spacing.y;

    let slot_frame = Frame::none()
        .fill(Color32::from_rgb(14, 14, 14))
        .inner_margin(Margin::same(12.0))
        .stroke(Stroke::new(1.0, Color32::from_rgb(28, 28, 28)));

    ui.horizontal(|ui| {
        ui.allocate_ui(egui::vec2(left_width, total_height), |ui| {
            ui.vertical(|ui| {
                ui.allocate_ui(egui::vec2(left_width, main_height), |ui| {
                    slot_frame.show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        render_main(ui);
                    });
                });

                ui.allocate_ui(egui::vec2(left_width, bottom_height), |ui| {
                    slot_frame.show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        render_bottom(ui);
                    });
                });
            });
        });

        ui.allocate_ui(egui::vec2(side_width, total_height), |ui| {
            slot_frame.show(ui, |ui| {
                ui.set_height(ui.available_height());
                ui.set_width(ui.available_width());
                render_side(ui);
            });
        });
    });
}

pub fn draw_nav_canvas_orchestrator<T>(
    ui: &mut Ui, 
    active_ticker: &str, 
    table_key: &str,          // "overview_metadata", "analysis_metadata", etc.
    heading_prefix: &str,     // "OVERVIEW", "VALUATION ENGINE", etc.
    id_source_key: &str,      // Unique string token for temporary UI state storage
    tabs: &[&dyn AbstractSubTab<T>]
) 
where 
    T: std::any::Any + Send + Sync, // Requirements to match backend memory slot contracts
{
    if tabs.is_empty() { return; }
    
    // Generate a uniquely distinct state ID token based on the caller context string
    let active_sub_tab_id = egui::Id::new(id_source_key);
    let current_tab_id = ui.data_mut(|d| d.get_temp::<usize>(active_sub_tab_id).unwrap_or(tabs[0].id()));

    let active_tab = tabs.iter().find(|t| t.id() == current_tab_id).unwrap_or(&tabs[0]);

    // Pull from the memory pool utilizing the exact concrete string key passed by the caller
    let table_found = backend::commands::memory_pool::with_active_table::<T, _, _>(table_key, |data| {
        draw_three_zone_canvas(
            ui,
            |ui| {
                ui.heading(format!("{}: {}", heading_prefix.to_uppercase(), active_ticker.to_uppercase()));
                ui.add_space(15.0);
                active_tab.render_main(ui, data);
            },
            |ui| {
                active_tab.render_bottom(ui, data);
            },
            |ui| {
                ui.vertical(|ui| {
                    let button_width = ui.available_width();
                    for tab in tabs {
                        if ui.add_sized(egui::vec2(button_width, 28.0), egui::Button::new(tab.label()).selected(current_tab_id == tab.id())).clicked() {
                            ui.data_mut(|d| d.insert_temp(active_sub_tab_id, tab.id()));
                        }
                        ui.add_space(4.0);
                    }
                });
            },
        );
    });

    // Uniform clean loading layout card fallback boundary execution
    if table_found.is_none() {
        draw_three_zone_canvas(
            ui,
            |ui| {
                ui.heading(format!("{}: {}", heading_prefix.to_uppercase(), active_ticker.to_uppercase()));
                ui.add_space(15.0);
                ui.weak("Loading data attributes into cache slot...");
            },
            |_ui| {},
            |_ui| {},
        );
    }
}

// =========================================================================
// HIGH-PERFORMANCE WORKSPACE CHART PAINTER
// =========================================================================

#[derive(Clone, Debug)]
pub struct GenericChartPoint {
    pub date: String,    // Format: "YYYY-MM-DD"
    pub value: f64,
}

#[derive(Clone, Debug)]
pub struct GenericChartLine {
    pub label: &'static str,
    pub color: Color32,
    pub stroke_width: f32,
    pub points: Vec<GenericChartPoint>, 
}

pub fn paint_abstract_chart_canvas(ui: &mut Ui, lines: &[GenericChartLine]) {
    // 1. Structural Validation Check
    if lines.is_empty() || lines.iter().map(|l| l.points.len()).sum::<usize>() == 0 {
        ui.centered_and_justified(|ui| {
            ui.weak("No historical metrics mapped to active canvas frame context.");
        });
        return;
    }

    // =========================================================================
    // COALESCE A UNIFIED ABSOLUTE TIME AXIS MATRIX
    // =========================================================================
    // Collect every single unique date across all lines to establish an absolute master timeline
    let mut timeline_set = BTreeSet::new();
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;

    for line in lines {
        for pt in &line.points {
            timeline_set.insert(pt.date.clone());
            if pt.value < min_val { min_val = pt.value; }
            if pt.value > max_val { max_val = pt.value; }
        }
    }

    // Flatten into a chronologically sorted linear reference array
    let master_timeline: Vec<String> = timeline_set.into_iter().collect();
    let total_timeline_ticks = master_timeline.len();

    if total_timeline_ticks < 2 { return; }

    // Map date string keys straight to their static lookup locations on our absolute matrix timeline
    let mut date_to_master_idx = HashMap::with_capacity(total_timeline_ticks);
    for (idx, date) in master_timeline.iter().enumerate() {
        date_to_master_idx.insert(date.clone(), idx);
    }

    // Dynamic padding adjustments over coordinate bounds
    if min_val >= max_val { min_val = 0.0; max_val = 100.0; }
    let value_range = max_val - min_val;
    min_val -= value_range * 0.05;
    max_val += value_range * 0.05;

    // Allocate frame size boundaries
    let desired_size = Vec2::new(ui.available_width(), ui.available_height() - 10.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 10, 10));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(25, 25, 25)));

    // =========================================================================
    // ABSOLUTE SCREEN SPACE PROJECTION MAP CLOSURES
    // =========================================================================
    // Coordinates map directly to their absolute structural layout index position on the master timeline grid
    let map_to_screen = |master_idx: usize, value: f64| -> Pos2 {
        let pct_x = (master_idx as f32) / ((total_timeline_ticks - 1) as f32);
        let screen_x = rect.left() + pct_x * rect.width();

        let pct_y = ((value - min_val) / (max_val - min_val)) as f32;
        let screen_y = rect.bottom() - (pct_y * rect.height());
        Pos2::new(screen_x, screen_y)
    };

    let map_to_master_index = |screen_x: f32| -> i32 {
        let pct_x = (screen_x - rect.left()) / rect.width();
        (pct_x * (total_timeline_ticks - 1) as f32).round() as i32
    };

    // Horizontal grid guidelines
    let grid_stroke = Stroke::new(1.0, Color32::from_rgb(20, 20, 20));
    for i in 1..4 {
        let y_pos = rect.top() + (rect.height() * 0.25 * (i as f32));
        painter.line_segment([Pos2::new(rect.left(), y_pos), Pos2::new(rect.right(), y_pos)], grid_stroke);
        
        let label_value = max_val - ((max_val - min_val) * 0.25 * (i as f64));
        painter.text(
            Pos2::new(rect.left() + 8.0, y_pos - 6.0),
            egui::Align2::LEFT_TOP,
            format!("₹ {:.1}", label_value),
            egui::FontId::proportional(11.0),
            Color32::from_rgb(100, 100, 100)
        );
    }

    // =========================================================================
    // YEAR-START ANCHORED MARKERS (ABSOLUTE PLACEMENTS)
    // =========================================================================
    let mut tracked_years = HashSet::new();
    for (idx, date) in master_timeline.iter().enumerate() {
        if date.len() >= 4 {
            let year_string = date[0..4].to_string();
            if !tracked_years.contains(&year_string) {
                tracked_years.insert(year_string.clone());
                
                let marker_pos = map_to_screen(idx, min_val);
                painter.line_segment(
                    [Pos2::new(marker_pos.x, rect.top()), Pos2::new(marker_pos.x, rect.bottom() - 20.0)],
                    Stroke::new(0.5, Color32::from_rgb(18, 18, 18))
                );
                painter.text(
                    Pos2::new(marker_pos.x, rect.bottom() - 6.0),
                    egui::Align2::CENTER_BOTTOM,
                    &year_string,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(70, 70, 70)
                );
            }
        }
    }

    // =========================================================================
    // PAINT SECURE DATA PATH VECTORS
    // =========================================================================
    let clip_rect = rect;
    for line in lines {
        if line.points.is_empty() { continue; }
        let stroke = Stroke::new(line.stroke_width, line.color);
        let mut last_screen_pos: Option<Pos2> = None;

        for pt in &line.points {
            // Find exactly where this date fits along our absolute timeline matrix map
            if let Some(&master_idx) = date_to_master_idx.get(&pt.date) {
                let current_screen_pos = map_to_screen(master_idx, pt.value);
                
                if let Some(prev_screen_pos) = last_screen_pos {
                    // Only stitch consecutive rows together to keep structural gaps from overlapping empty years
                    painter.with_clip_rect(clip_rect).line_segment([prev_screen_pos, current_screen_pos], stroke);
                }
                last_screen_pos = Some(current_screen_pos);
            }
        }
    }

    // =========================================================================
    // ABSOLUTE RADAR HUD DISPLAY LAYER
    // =========================================================================
    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
        if rect.contains(pointer_pos) {
            let ref_idx = map_to_master_index(pointer_pos.x).clamp(0, (total_timeline_ticks - 1) as i32) as usize;
            let current_timeline_date = &master_timeline[ref_idx];

            // Vertical guide line
            painter.line_segment(
                [Pos2::new(pointer_pos.x, rect.top()), Pos2::new(pointer_pos.x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
            );

            // Timeline Date Badge
            let date_badge_rect = egui::Rect::from_center_size(
                Pos2::new(pointer_pos.x, rect.bottom() - 14.0),
                Vec2::new(75.0, 18.0)
            );
            painter.rect_filled(date_badge_rect, 2.0, Color32::from_rgb(30, 30, 30));
            painter.text(
                date_badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                current_timeline_date,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(255, 255, 255)
            );

            // Left-Axis Value Tags Layout Loop
            let mut vertical_stack_offset = 0.0;
            for line in lines {
                if line.points.is_empty() { continue; }

                let target_point = line.points.iter()
                    .filter(|p| p.date <= *current_timeline_date)
                    .last() 
                    .or_else(|| line.points.first()); 

                if let Some(pt) = target_point {
                    let screen_pos = map_to_screen(ref_idx, pt.value);
                    painter.circle_filled(screen_pos, 4.0, line.color);

                    let label_text = format!("{}: ₹{:.1}", line.label, pt.value);
                    let tag_rect = egui::Rect::from_min_size(
                        Pos2::new(rect.left() + 4.0, screen_pos.y - 9.0 + vertical_stack_offset),
                        Vec2::new(120.0, 18.0) 
                    );

                    painter.rect_filled(tag_rect, 2.0, line.color);
                    painter.text(
                        tag_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label_text,
                        egui::FontId::proportional(10.0),
                        if line.color == Color32::from_rgb(250, 210, 50) { Color32::BLACK } else { Color32::WHITE }
                    );
                    vertical_stack_offset += 22.0;
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenericBarGroup {
    pub date: String,    // Format: "YYYY-MM-DD" or "YYYY"
    pub value: f64,      
    pub label: String,    
}

#[derive(Clone, Debug)]
pub struct GenericBarChartSeries {
    pub series_name: &'static str,
    pub positive_color: Color32,  // Color for values > 0 (e.g., Wealth Creation Green)
    pub negative_color: Color32,  // Color for values < 0 (e.g., Capital Destruction Red)
    pub groups: Vec<GenericBarGroup>, // Presorted chronologically
}

/// Abstract canvas that computes a floating zero-axis line to draw clean upward/downward bars
pub fn paint_abstract_bar_canvas(ui: &mut Ui, series: &GenericBarChartSeries) {
    if series.groups.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak("No historical data available for active bar chart frame context.");
        });
        return;
    }

    // =========================================================================
    // INITIALIZE BOUNDS AND Y-AXIS SCALE
    // =========================================================================
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;

    for bar in &series.groups {
        if bar.value < min_val { min_val = bar.value; }
        if bar.value > max_val { max_val = bar.value; }
    }

    if min_val > 0.0 { min_val = 0.0; }
    if max_val < 0.0 { max_val = 0.0; }

    let value_range = max_val - min_val;
    let buffer = if value_range == 0.0 { 100.0 } else { value_range * 0.10 };
    max_val += buffer;
    min_val -= buffer;

    let desired_size = Vec2::new(ui.available_width(), ui.available_height() - 10.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 10, 10));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(25, 25, 25)));

    // =========================================================================
    // SCREEN SPACE MAPPING & HIGH-DENSITY AXIS GUIDELINES
    // =========================================================================
    let total_bars = series.groups.len();
    
    let map_y = |val: f64| -> f32 {
        let pct_y = ((val - min_val) / (max_val - min_val)) as f32;
        rect.bottom() - (pct_y * rect.height())
    };

    let zero_screen_y = map_y(0.0);

    // High-Density Horizontal Grid (7 benchmark splits instead of 3 for better readability)
    let grid_stroke = Stroke::new(1.0, Color32::from_rgb(20, 20, 20));
    let splits = 8; // Creates 7 lines inside the bounds
    for i in 1..splits {
        let pct = i as f64 / splits as f64;
        let y_pos = rect.top() + (rect.height() * pct as f32);
        
        painter.line_segment([Pos2::new(rect.left(), y_pos), Pos2::new(rect.right(), y_pos)], grid_stroke);
        
        let label_value = max_val - (value_range * pct);
        painter.text(
            Pos2::new(rect.left() + 8.0, y_pos - 6.0),
            egui::Align2::LEFT_TOP,
            format!("₹ {:.1}", label_value),
            egui::FontId::proportional(10.0),
            Color32::from_rgb(90, 90, 90)
        );
    }

    // =========================================================================
    // GEOMETRIC BAR RENDERING
    // =========================================================================
    let total_width = rect.width();
    let section_width = total_width / (total_bars as f32);
    let bar_width = section_width * 0.75;
    let half_bar = bar_width / 2.0;

    for (idx, bar) in series.groups.iter().enumerate() {
        let center_x = rect.left() + (idx as f32 * section_width) + (section_width / 2.0);
        let target_y = map_y(bar.value);

        let bar_rect = if bar.value >= 0.0 {
            Rect::from_min_max(Pos2::new(center_x - half_bar, target_y), Pos2::new(center_x + half_bar, zero_screen_y))
        } else {
            Rect::from_min_max(Pos2::new(center_x - half_bar, zero_screen_y), Pos2::new(center_x + half_bar, target_y))
        };

        let fill_color = if bar.value >= 0.0 { series.positive_color } else { series.negative_color };
        painter.rect_filled(bar_rect, 2.0, fill_color);

        if bar.date.len() >= 4 {
            let year_text = bar.date[0..4].to_string();
            painter.text(
                Pos2::new(center_x, rect.bottom() - 6.0),
                egui::Align2::CENTER_BOTTOM,
                year_text,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(100, 100, 100)
            );
        }
    }

    // Dynamic Zero Intercept
    painter.line_segment(
        [Pos2::new(rect.left(), zero_screen_y), Pos2::new(rect.right(), zero_screen_y)],
        Stroke::new(1.5, Color32::from_rgb(70, 70, 70))
    );

    // =========================================================================
    // ABSOLUTE RADAR HUD DISPLAY LAYER
    // =========================================================================
    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
        if rect.contains(pointer_pos) {
            let pct_x = (pointer_pos.x - rect.left()) / rect.width();
            let hover_idx = (pct_x * total_bars as f32).floor().clamp(0.0, (total_bars - 1) as f32) as usize;

            if let Some(target_bar) = series.groups.get(hover_idx) {
                let center_x = rect.left() + (hover_idx as f32 * section_width) + (section_width / 2.0);
                let target_y = map_y(target_bar.value);

                // 1. Vertical Guide Line
                painter.line_segment(
                    [Pos2::new(center_x, rect.top()), Pos2::new(center_x, rect.bottom() - 20.0)],
                    Stroke::new(1.0, Color32::from_rgb(80, 80, 80)) // Slightly brighter for visibility
                );

                // 2. Focused Target Dot
                painter.circle_filled(Pos2::new(center_x, target_y), 4.0, Color32::WHITE);

                // 3. Optional Context Label (Centered above the bar)
                if !target_bar.label.trim().is_empty() {
                    let text_color = if target_bar.value >= 0.0 { series.positive_color } else { series.negative_color };
                    painter.text(
                        Pos2::new(center_x, target_y - 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        &target_bar.label,
                        egui::FontId::proportional(12.0),
                        text_color
                    );
                }

                // 4. Timeline Date Badge
                let date_badge_rect = egui::Rect::from_center_size(
                    Pos2::new(center_x, rect.bottom() - 14.0),
                    Vec2::new(75.0, 18.0)
                );
                painter.rect_filled(date_badge_rect, 2.0, Color32::from_rgb(40, 40, 40));
                painter.text(
                    date_badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &target_bar.date,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(255, 255, 255)
                );

                // 5. Aligned Axis Metric Box (Distinct Blue HUD Color to avoid blending into the bar)
                let hud_bg_color = Color32::from_rgb(40, 120, 220); // Deep Sky Blue 
                let label_text = format!("{}: ₹ {:.1}", series.series_name, target_bar.value);

                let tag_rect = egui::Rect::from_min_size(
                    Pos2::new(rect.left() + 4.0, target_y - 9.0),
                    Vec2::new(140.0, 18.0)
                );

                painter.rect_filled(tag_rect, 2.0, hud_bg_color);
                painter.text(
                    tag_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label_text,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE
                );
            }
        }
    }
}