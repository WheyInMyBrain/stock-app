// stock-app/ui/frontend_native/src/ui/theme.rs
use egui::{Color32, Context, Stroke, Visuals, TextStyle, FontId, FontFamily};

pub fn apply_high_contrast_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();

    // Lock backgrounds to absolute black
    visuals.window_fill = Color32::from_rgb(0, 0, 0);
    visuals.panel_fill = Color32::from_rgb(0, 0, 0);
    visuals.extreme_bg_color = Color32::from_rgb(0, 0, 0);
    visuals.faint_bg_color = Color32::from_rgb(15, 15, 15);

    // Text & Primary Foreground States
    visuals.override_text_color = Some(Color32::from_rgb(255, 255, 255));
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(255, 255, 255);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(200, 200, 200);
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(255, 255, 255);
    visuals.widgets.active.fg_stroke.color = Color32::from_rgb(255, 255, 255);

    // Widget Fills
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0, 0, 0);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(25, 25, 25);
    visuals.widgets.active.bg_fill = Color32::from_rgb(40, 40, 40);

    // Flat Border Outlines
    let white_stroke = Stroke::new(1.0, Color32::from_rgb(255, 255, 255));
    let faint_stroke = Stroke::new(1.0, Color32::from_rgb(60, 60, 60));
    
    visuals.widgets.inactive.bg_stroke = white_stroke;
    visuals.widgets.hovered.bg_stroke = white_stroke;
    visuals.widgets.active.bg_stroke = white_stroke;
    visuals.widgets.noninteractive.bg_stroke = faint_stroke;

    // Zero out rounded shapes & shadow allocations
    visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
    visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
    visuals.widgets.active.rounding = egui::Rounding::ZERO;
    visuals.window_rounding = egui::Rounding::ZERO;
    visuals.window_shadow = egui::epaint::Shadow::NONE;

    ctx.set_visuals(visuals);

    // Apply strict monospaced layout grid font
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Monospace)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(10.0, FontFamily::Monospace)),
    ].into();
    ctx.set_style(style);
}