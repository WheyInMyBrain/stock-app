// stock-app/ui/frontend/src/ui/panels/ai.rs

use egui::{Color32, RichText, ScrollArea, Stroke, Ui, Align, Layout};
use std::sync::{Mutex, LazyLock};
use tokio::sync::mpsc::UnboundedReceiver;

use backend::commands::ai::run_ai_analyst_stream;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub is_user: bool,
    pub content: String,
}

pub struct ActiveAIState {
    pub conversation: Vec<ChatMessage>,
    pub input_buffer: String,
    pub active_receiver: Option<UnboundedReceiver<String>>,
    pub is_generating: bool,
}

pub static AI_PANEL_STATE: LazyLock<Mutex<ActiveAIState>> = LazyLock::new(|| {
    Mutex::new(ActiveAIState {
        conversation: vec![ChatMessage {
            is_user: false,
            content: "Welcome to the Analysis Terminal. Ask me anything regarding historical financial indicators or trends for the active stock symbol.".to_string(),
        }],
        input_buffer: String::new(),
        active_receiver: None,
        is_generating: false,
    })
});

pub fn draw_ai_panel(ui: &mut Ui, _active_ticker: &str) {
    let mut state = match AI_PANEL_STATE.lock() {
        Ok(guard) => guard,
        Err(_) => {
            ui.label(RichText::new("❌ State contention lock error.").color(Color32::RED));
            return;
        }
    };

    let mut pending_tokens = Vec::new();
    if let Some(ref mut receiver) = state.active_receiver {
        while let Ok(token) = receiver.try_recv() {
            pending_tokens.push(token);
        }
    }

    if !pending_tokens.is_empty() {
        if let Some(last_message) = state.conversation.last_mut() {
            if !last_message.is_user {
                for token in pending_tokens {
                    last_message.content.push_str(&token);
                }
            }
        }
    }

    let total_height = ui.available_height();
    let bottom_input_area_height = 45.0;
    let main_chat_area_height = total_height - bottom_input_area_height - ui.spacing().item_spacing.y;

    // =================================================================
    // 1. TOP ZONE: Chat Feed History
    // =================================================================
    ui.allocate_ui(egui::vec2(ui.available_width(), main_chat_area_height), |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true) 
            .show(ui, |ui| {
                ui.add_space(4.0);
                
                for message in &state.conversation {
                    let (layout, background_color, border_color) = if message.is_user {
                        (
                            Layout::top_down(Align::Max), 
                            Color32::from_rgb(32, 45, 36), 
                            Color32::from_rgb(50, 80, 60),
                        )
                    } else {
                        (
                            Layout::top_down(Align::Min), 
                            Color32::from_rgb(22, 22, 22), 
                            Color32::from_rgb(44, 44, 44),
                        )
                    };

                    ui.with_layout(layout, |ui| {
                        let max_bubble_width = ui.available_width() * 0.80;
                        
                        egui::Frame::none()
                            .fill(background_color)
                            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
                            .stroke(Stroke::new(1.0, border_color))
                            .rounding(8.0)
                            .show(ui, |ui| {
                                ui.set_max_width(max_bubble_width);
                                render_smart_payload(ui, &message.content);
                            });
                    });
                }
                ui.add_space(4.0);
            });
    });

    ui.add_space(ui.spacing().item_spacing.y);

    // =================================================================
    // 2. BOTTOM ZONE: Input Bar
    // =================================================================
    ui.allocate_ui(egui::vec2(ui.available_width(), bottom_input_area_height), |ui| {
        ui.horizontal(|ui| {
            let input_width = ui.available_width() - 85.0;

            let response = ui.add(
                egui::TextEdit::singleline(&mut state.input_buffer)
                    .hint_text("Ask a financial strategy question...")
                    .desired_width(input_width)
                    .margin(egui::Margin::same(8.0))
            );

            let submit_intent = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            
            ui.add_enabled_ui(!state.is_generating, |ui| {
                if (ui.button("⚡ Ask").clicked() || submit_intent) && !state.input_buffer.trim().is_empty() {
                    let clean_query = state.input_buffer.trim().to_string();
                    state.input_buffer.clear();

                    state.conversation.push(ChatMessage {
                        is_user: true,
                        content: clean_query.clone(),
                    });

                    state.conversation.push(ChatMessage {
                        is_user: false,
                        content: String::new(),
                    });

                    state.is_generating = true;
                    ui.ctx().request_repaint(); 

                    let ctx_clone = ui.ctx().clone();
                    
                    tokio::spawn(async move {
                        match run_ai_analyst_stream(clean_query).await {
                            Ok(rx) => {
                                if let Ok(mut guard) = AI_PANEL_STATE.lock() {
                                    guard.active_receiver = Some(rx);
                                }
                            }
                            Err(err_msg) => {
                                if let Ok(mut guard) = AI_PANEL_STATE.lock() {
                                    if let Some(last_msg) = guard.conversation.last_mut() {
                                        last_msg.content = format!("❌ Sidecar Process Error: {}", err_msg);
                                    }
                                    guard.is_generating = false;
                                }
                            }
                        }

                        tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                if let Ok(mut guard) = AI_PANEL_STATE.lock() {
                                    if let Some(ref rx) = guard.active_receiver {
                                        if rx.is_closed() {
                                            guard.active_receiver = None;
                                            guard.is_generating = false;
                                            ctx_clone.request_repaint();
                                            break;
                                        }
                                    } else {
                                        guard.is_generating = false;
                                        break;
                                    }
                                }
                                ctx_clone.request_repaint(); 
                            }
                        });
                    });
                }
            });
        });
    });
}

/// 🧠 STATE-BASED BOUNDARY PARSER: Strips tags and groups tokens cleanly by their output zone
fn render_smart_payload(ui: &mut Ui, content: &str) {
    let mut thinking_text = String::new();
    let mut main_text = String::new();
    
    let mut current_mode = "main"; // default state

    // 🚀 THE PARSER LOOP: Split the stdout chunk dynamically based on your C++ delimiter boundaries
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if current_mode == "main" { main_text.push_str("\n"); }
            if current_mode == "think" { thinking_text.push_str("\n"); }
            continue;
        }

        if trimmed == "__THINKING_OUTPUT_START__" {
            current_mode = "think";
            continue;
        } else if trimmed == "__THINKING_OUTPUT_END__" {
            current_mode = "main";
            continue;
        } else if trimmed == "__MAIN_OUTPUT_START__" {
            current_mode = "main";
            continue;
        } else if trimmed == "__MAIN_OUTPUT_END__" || trimmed == "__DATA_OUTPUT_START__" || trimmed == "__DATA_OUTPUT_END__" {
            continue;
        }

        if current_mode == "think" {
            thinking_text.push_str(line);
            thinking_text.push_str("\n");
        } else {
            main_text.push_str(line);
            main_text.push_str("\n");
        }
    }

    // Always draw the thinking dropdown pinned cleanly at the absolute top if thoughts are present
    if !thinking_text.trim().is_empty() {
        ui.add_space(2.0);
        egui::CollapsingHeader::new(RichText::new("💭 Thinking Process").color(Color32::from_rgb(130, 130, 130)).italics())
            .default_open(false)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for line in thinking_text.lines() {
                        let t_line = line.trim();
                        if t_line.is_empty() || (t_line.starts_with("20") && t_line.contains('-') && t_line.len() <= 10) { continue; }
                        ui.label(RichText::new(t_line).color(Color32::from_rgb(140, 140, 140)));
                    }
                });
            });
        ui.add_space(4.0);
    } else if content.contains("__THINKING_OUTPUT_START__") && !content.contains("__THINKING_OUTPUT_END__") {
        // Model is actively streaming reasoning tokens right now
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Thinking...").color(Color32::from_rgb(130, 130, 130)).italics());
        });
        ui.add_space(4.0);
    }

    // Render the final cleaned text explanation directly underneath
    if !main_text.trim().is_empty() {
        parse_and_draw_markdown_payload(ui, &main_text);
    }
}

fn parse_and_draw_markdown_payload(ui: &mut Ui, text: &str) {
    if text.contains('|') && text.contains('\n') {
        egui::Grid::new(ui.next_auto_id())
            .striped(true)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for line in text.lines() {
                    if line.contains("---") || line.trim().is_empty() { continue; }
                    for cell in line.split('|') {
                        let trimmed = cell.trim();
                        if trimmed.starts_with("**") && trimmed.ends_with("**") {
                            ui.label(RichText::new(&trimmed[2..trimmed.len()-2]).strong().color(Color32::WHITE));
                        } else {
                            ui.label(RichText::new(trimmed).color(Color32::from_rgb(210, 210, 210)));
                        }
                    }
                    ui.end_row();
                }
            });
        return;
    }

    ui.spacing_mut().item_spacing.y = 4.0;

    ui.vertical(|ui| {
        for line in text.lines() {
            let mut remaining = line.trim();
            if remaining.is_empty() { continue; }

            // Double check to drop out remaining raw C++ header string scraps
            if remaining.contains("Routing request pipeline") 
                || remaining.contains("PART 1") 
                || remaining.contains("Directing data loader") 
                || remaining.contains("PART 2") 
                || remaining.contains("PART 3")
                || remaining.starts_with("📈 ---")
                || remaining.starts_with("🤖 ---")
                || (remaining.starts_with("20") && remaining.contains('-') && remaining.len() <= 12) 
            {
                continue;
            }

            ui.horizontal_wrapped(|ui| {
                if remaining.starts_with("* ") || remaining.starts_with("- ") {
                    ui.label(RichText::new("•").strong().color(Color32::from_rgb(0, 220, 130)));
                    remaining = &remaining[2..];
                } else if remaining.chars().next().map_or(false, |c| c.is_ascii_digit()) && remaining.contains(". ") {
                    if let Some(dot_idx) = remaining.find(". ") {
                        let num_prefix = &remaining[..dot_idx + 2];
                        ui.label(RichText::new(num_prefix).strong().color(Color32::from_rgb(0, 220, 130)));
                        remaining = &remaining[dot_idx + 2..];
                    }
                }

                while let Some(start_idx) = remaining.find("**") {
                    if let Some(end_idx) = remaining[start_idx + 2..].find("**") {
                        let absolute_end = start_idx + 2 + end_idx;
                        
                        if start_idx > 0 {
                            ui.label(RichText::new(&remaining[..start_idx]).color(Color32::from_rgb(230, 230, 230)));
                        }
                        ui.label(RichText::new(&remaining[start_idx + 2..absolute_end]).strong().color(Color32::WHITE));
                        
                        remaining = &remaining[absolute_end + 2..];
                    } else {
                        break;
                    }
                }
                
                if !remaining.is_empty() {
                    ui.label(RichText::new(remaining).color(Color32::from_rgb(230, 230, 230)));
                }
            });
        }
    });
}