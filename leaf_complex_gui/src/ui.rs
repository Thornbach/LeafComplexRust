// UI Rendering Components
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, Points};
use std::sync::{Arc, Mutex};

use crate::state::{AppState, AnalysisStatus};

const PINK_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 255);
const YELLOW_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);
const RED_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);

/// Render the main image view panel with zoom/pan
pub fn render_image_view(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>, _ctx: &egui::Context) {
    let state_guard = state.lock().unwrap();
    
    ui.heading("🖼️ Image View");
    ui.separator();
    
    // Controls
    ui.horizontal(|ui| {
        if ui.button("Reset View").clicked() {
            drop(state_guard);
            state.lock().unwrap().reset_view();
            return;
        }
        
        ui.label(format!("Zoom: {:.0}%", state_guard.zoom_level * 100.0));
        
        ui.separator();
        
        let mut show_ec = state_guard.show_ec_overlay;
        let mut show_mc = state_guard.show_mc_overlay;
        
        drop(state_guard);
        
        if ui.checkbox(&mut show_ec, "EC Overlay").changed() {
            state.lock().unwrap().show_ec_overlay = show_ec;
        }
        if ui.checkbox(&mut show_mc, "MC Overlay").changed() {
            state.lock().unwrap().show_mc_overlay = show_mc;
        }
    });
    
    ui.separator();
    
    // Image display area
    let available_size = ui.available_size();
    let (response, painter) = ui.allocate_painter(available_size, egui::Sense::drag());
    
    let state_guard = state.lock().unwrap();
    
    if let Some(result) = state_guard.current_result() {
        // Render base image
        if let Some(texture) = &result.original_texture {
            let tex_size = texture.size_vec2();
            let zoom = state_guard.zoom_level;
            let offset = state_guard.pan_offset;
            
            let image_size = tex_size * zoom;
            let rect = egui::Rect::from_center_size(
                response.rect.center() + offset,
                image_size,
            );
            
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            
            // Render overlays
            if state_guard.show_ec_overlay {
                if let Some(ec_texture) = &result.ec_image_texture {
                    painter.image(
                        ec_texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            
            if state_guard.show_mc_overlay {
                if let Some(mc_texture) = &result.mc_image_texture {
                    painter.image(
                        mc_texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
            }
            
            // Highlight selected point
            if let Some(_point_idx) = state_guard.selected_point {
                // Draw red circle at selected point
                // Note: We'd need contour point positions to do this properly
                // For now, just show that selection is active
                painter.circle_filled(
                    response.rect.center(),
                    5.0,
                    RED_COLOR,
                );
            }
        }
    } else {
        // No analysis result yet
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            if state_guard.current_image().is_some() {
                "Click 'Analyze Current Image' to start"
            } else {
                "Open a workspace folder to begin"
            },
            egui::FontId::proportional(16.0),
            ui.visuals().text_color(),
        );
    }
    
    // Handle pan and zoom
    drop(state_guard);
    
    if response.dragged() {
        let mut state_guard = state.lock().unwrap();
        state_guard.pan_offset += response.drag_delta();
    }
    
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let mut state_guard = state.lock().unwrap();
            let zoom_delta = scroll * 0.001;
            state_guard.zoom_level = (state_guard.zoom_level + zoom_delta).clamp(0.1, 5.0);
        }
    }
}

/// Render the EC graph panel
pub fn render_ec_graph(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>) {
    ui.heading("📊 Edge Complexity (EC)");
    
    let (data, selected_point) = {
        let state_guard = state.lock().unwrap();
        if let Some(result) = state_guard.current_result() {
            (result.ec_data.clone(), state_guard.selected_point)
        } else {
            ui.label("No analysis data available");
            return;
        }
    };
        
        let points: PlotPoints = data.iter()
            .map(|&(x, y)| [x, y])
            .collect();
        
        let line = Line::new(points)
            .color(PINK_COLOR)
            .width(2.0)
            .name("Geodesic_EC");
        
        let plot = Plot::new("ec_plot")
            .height(200.0)
            .legend(egui_plot::Legend::default())
            .x_axis_label("Point Index")
            .y_axis_label("Geodesic EC");
        
        let response = plot.show(ui, |plot_ui| {
            plot_ui.line(line);
            
            // Highlight selected point
            if let Some(idx) = selected_point {
                if let Some(&(x, y)) = data.get(idx) {
                    let highlight = Points::new(vec![[x, y]])
                        .color(RED_COLOR)
                        .radius(5.0)
                        .name("Selected");
                    plot_ui.points(highlight);
                }
            }
        });
        
        // Check for point selection in graph
        if let Some(pointer_pos) = response.response.hover_pos() {
            if response.response.clicked() {
                let plot_pos = response.transform.value_from_position(pointer_pos);
                let clicked_x = plot_pos.x;
                let closest_idx = data.iter()
                    .enumerate()
                    .min_by_key(|(_, (x, _))| ((x - clicked_x).abs() * 1000.0) as i32)
                    .map(|(idx, _)| idx);
                
                if let Some(idx) = closest_idx {
                    state.lock().unwrap().selected_point = Some(idx);
                }
            }
        }
    
}

/// Render the MC graph panel
pub fn render_mc_graph(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>) {
    ui.heading("📊 Margin Complexity (MC)");
    
    let (data, selected_point) = {
        let state_guard = state.lock().unwrap();
        if let Some(result) = state_guard.current_result() {
            (result.mc_data.clone(), state_guard.selected_point)
        } else {
            ui.label("No analysis data available");
            return;
        }
    };
        
        let points: PlotPoints = data.iter()
            .map(|&(x, y)| [x, y])
            .collect();
        
        let line = Line::new(points)
            .color(YELLOW_COLOR)
            .width(2.0)
            .name("Geodesic_MC_H");
        
        let plot = Plot::new("mc_plot")
            .height(200.0)
            .legend(egui_plot::Legend::default())
            .x_axis_label("Point Index")
            .y_axis_label("Geodesic MC (Harmonic)");
        
        let response = plot.show(ui, |plot_ui| {
            plot_ui.line(line);
            
            // Highlight selected point
            if let Some(idx) = selected_point {
                if let Some(&(x, y)) = data.get(idx) {
                    let highlight = Points::new(vec![[x, y]])
                        .color(RED_COLOR)
                        .radius(5.0)
                        .name("Selected");
                    plot_ui.points(highlight);
                }
            }
        });
        
        // Check for point selection in graph
        if let Some(pointer_pos) = response.response.hover_pos() {
            if response.response.clicked() {
                let plot_pos = response.transform.value_from_position(pointer_pos);
                let clicked_x = plot_pos.x;
                let closest_idx = data.iter()
                    .enumerate()
                    .min_by_key(|(_, (x, _))| ((x - clicked_x).abs() * 1000.0) as i32)
                    .map(|(idx, _)| idx);
                
                if let Some(idx) = closest_idx {
                    state.lock().unwrap().selected_point = Some(idx);
                }
            }
        }
    
}

/// Render summary statistics panel
pub fn render_summary_panel(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>) {
    ui.heading("📋 Summary Statistics");
    ui.separator();
    
    let state_guard = state.lock().unwrap();
    
    if let Some(result) = state_guard.current_result() {
        let summary = &result.summary;
        
        egui::Grid::new("summary_grid")
            .num_columns(4)
            .spacing([20.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                // Headers
                ui.label("");
                ui.strong("EC");
                ui.strong("MC");
                ui.label("");
                ui.end_row();
                
                // Length
                ui.label("Length:");
                ui.label(format!("{:.1} px", summary.ec_length));
                ui.label(format!("{:.1} px", summary.mc_length));
                ui.label("");
                ui.end_row();
                
                // Width
                ui.label("Width:");
                ui.label(format!("{:.1} px", summary.ec_width));
                ui.label(format!("{:.1} px", summary.mc_width));
                ui.label("");
                ui.end_row();
                
                // Shape Index
                ui.label("Shape Index:");
                ui.label(format!("{:.3}", summary.ec_shape_index));
                ui.label(format!("{:.3}", summary.mc_shape_index));
                ui.label("");
                ui.end_row();
                
                // Spectral Entropy
                ui.label("Complexity:");
                ui.label(format!("{:.4}", summary.ec_spectral_entropy));
                ui.label(format!("{:.4}", summary.mc_spectral_entropy));
                ui.label("");
                ui.end_row();
                
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.end_row();
                
                // Shared stats
                ui.label("Area:");
                ui.label(format!("{} px²", summary.area));
                ui.label("");
                ui.label("");
                ui.end_row();
                
                ui.label("Outline Count:");
                ui.label(format!("{}", summary.outline_count));
                ui.label("");
                ui.label("");
                ui.end_row();
                
                ui.label("Harmonic Chains:");
                ui.label(format!("{}", summary.harmonic_chain_count));
                ui.label("");
                ui.label("");
                ui.end_row();
            });
    } else {
        ui.label("No analysis data available");
    }
}

/// Render combined analysis panel (graphs + stats)
pub fn render_analysis_panel(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>, _ctx: &egui::Context) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // EC Graph
        render_ec_graph(ui, state);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        
        // MC Graph
        render_mc_graph(ui, state);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        
        // Summary Stats
        render_summary_panel(ui, state);
    });
}

/// Render thumbnail strip at bottom
pub fn render_thumbnail_strip(ui: &mut egui::Ui, state: &Arc<Mutex<AppState>>) {
    let state_guard = state.lock().unwrap();
    let mut selected_idx: Option<usize> = None;
    
    ui.horizontal(|ui| {
        ui.heading("📂 Workspace Images");
        ui.label(format!("({} images)", state_guard.images.len()));
    });
    
    ui.separator();
    
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let current_idx = state_guard.current_image_index;
                
                for (idx, img_info) in state_guard.images.iter().enumerate() {
                    let is_selected = current_idx == Some(idx);
                    
                    let frame = if is_selected {
                        egui::Frame::default()
                            .fill(ui.visuals().selection.bg_fill)
                            .stroke(ui.visuals().selection.stroke)
                            .rounding(4.0)
                            .inner_margin(4.0)
                    } else {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .rounding(4.0)
                            .inner_margin(4.0)
                    };
                    
                    frame.show(ui, |ui| {
                        ui.set_width(100.0);
                        ui.set_height(80.0);
                        
                        ui.vertical_centered(|ui| {
                            // Thumbnail placeholder
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(90.0, 60.0),
                                egui::Sense::click(),
                            );
                            
                            if let Some(thumbnail) = &img_info.thumbnail {
                                ui.painter().image(
                                    thumbnail.id(),
                                    rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                ui.painter().rect_filled(rect, 0.0, egui::Color32::DARK_GRAY);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🖼️",
                                    egui::FontId::proportional(20.0),
                                    egui::Color32::GRAY,
                                );
                            }
                            
                            if response.clicked() {
                                selected_idx = Some(idx);
                            }
                            
                            // Status indicator
                            let status_text = match img_info.status {
                                AnalysisStatus::NotStarted => "⭕",
                                AnalysisStatus::Running => "⏳",
                                AnalysisStatus::Completed => "✅",
                                AnalysisStatus::Failed => "❌",
                            };
                            
                            ui.label(status_text);
                            
                            // Filename (truncated)
                            let filename = if img_info.filename.len() > 12 {
                                format!("{}...", &img_info.filename[..9])
                            } else {
                                img_info.filename.clone()
                            };
                            ui.label(filename);
                        });
                    });
                }
            });
        });
    
    drop(state_guard);
    
    if let Some(idx) = selected_idx {
        state.lock().unwrap().select_image(idx);
    }
}
