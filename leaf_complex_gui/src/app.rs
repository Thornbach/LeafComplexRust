// Main Application Structure
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::state::{AppState, AnalysisStatus};
use crate::ui;
use crate::analysis::AnalysisEngine;
use crate::config_editor::ConfigEditor;
use leaf_complex_rust_lib::Config;

pub struct LeafComplexApp {
    state: Arc<Mutex<AppState>>,
    config: Arc<Mutex<Config>>,
    analysis_engine: AnalysisEngine,
    config_editor: ConfigEditor,
    show_config_editor: bool,
}

impl LeafComplexApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load config from file or create default
        let config = Config::from_file("config.toml")
            .unwrap_or_else(|_| {
                eprintln!("Could not load config.toml, using defaults");
                Config::default()
            });
        
        Self {
            state: Arc::new(Mutex::new(AppState::default())),
            config: Arc::new(Mutex::new(config.clone())),
            analysis_engine: AnalysisEngine::new(),
            config_editor: ConfigEditor::new(config),
            show_config_editor: false,
        }
    }
    
    fn render_menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("📁 Open Workspace...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Workspace Folder")
                        .pick_folder()
                    {
                        let mut state = self.state.lock().unwrap();
                        state.load_workspace(path);
                        drop(state);
                        
                        // Start analysis on first image
                        self.analyze_current_image(ctx);
                        ui.close_menu();
                    }
                }
                
                ui.separator();
                
                if ui.button("💾 Export Current Analysis...").clicked() {
                    self.export_current_analysis();
                    ui.close_menu();
                }
                
                if ui.button("🖼️ Save Annotated Image...").clicked() {
                    self.save_annotated_image();
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("❌ Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
            ui.menu_button("View", |ui| {
                let mut state = self.state.lock().unwrap();
                
                ui.checkbox(&mut state.show_ec_overlay, "Show EC Overlay");
                ui.checkbox(&mut state.show_mc_overlay, "Show MC Overlay");
                
                ui.separator();
                
                if ui.button("🔍 Reset Zoom").clicked() {
                    state.reset_view();
                    ui.close_menu();
                }
            });
            
            ui.menu_button("Analysis", |ui| {
                if ui.button("▶️ Analyze Current Image").clicked() {
                    self.analyze_current_image(ctx);
                    ui.close_menu();
                }
                
                if ui.button("▶️▶️ Analyze All Images").clicked() {
                    self.analyze_all_images(ctx);
                    ui.close_menu();
                }
                
                ui.separator();
                
                if ui.button("⚙️ Configuration...").clicked() {
                    self.show_config_editor = true;
                    ui.close_menu();
                }
            });
            
            ui.menu_button("Help", |ui| {
                if ui.button("📖 Documentation").clicked() {
                    // Open documentation
                    ui.close_menu();
                }
                
                if ui.button("ℹ️ About").clicked() {
                    // Show about dialog
                    ui.close_menu();
                }
            });
        });
    }
    
    fn analyze_current_image(&mut self, ctx: &egui::Context) {
        let state = Arc::clone(&self.state);
        let config = Arc::clone(&self.config);
        let ctx = ctx.clone();
        
        // Get current image path
        let image_path = {
            let state_guard = state.lock().unwrap();
            if let Some(img) = state_guard.current_image() {
                Some(img.path.clone())
            } else {
                None
            }
        };
        
        if let Some(path) = image_path {
            // Mark as in progress
            {
                let mut state_guard = state.lock().unwrap();
                state_guard.analysis_in_progress = true;
                if let Some(idx) = state_guard.current_image_index {
                    if let Some(img) = state_guard.images.get_mut(idx) {
                        img.status = AnalysisStatus::Running;
                    }
                }
            }
            
            // Run analysis in background thread
            thread::spawn(move || {
                let engine = AnalysisEngine::new();
                let config_guard = config.lock().unwrap();
                
                match engine.analyze_image(&path, &config_guard, &ctx) {
                    Ok(result) => {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.analysis_results.insert(path.clone(), result);
                        state_guard.analysis_in_progress = false;
                        
                        if let Some(idx) = state_guard.current_image_index {
                            if let Some(img) = state_guard.images.get_mut(idx) {
                                if img.path == path {
                                    img.status = AnalysisStatus::Completed;
                                }
                            }
                        }
                        
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.last_error = Some(format!("Analysis failed: {}", e));
                        state_guard.analysis_in_progress = false;
                        
                        if let Some(idx) = state_guard.current_image_index {
                            if let Some(img) = state_guard.images.get_mut(idx) {
                                if img.path == path {
                                    img.status = AnalysisStatus::Failed;
                                }
                            }
                        }
                        
                        ctx.request_repaint();
                    }
                }
            });
        }
    }
    
    fn analyze_all_images(&mut self, _ctx: &egui::Context) {
        // Similar to analyze_current_image but for all images
        // Would iterate and analyze each one
        eprintln!("Analyze all images not yet implemented");
    }
    
    fn export_current_analysis(&mut self) {
        // Export graphs as PNG/SVG
        eprintln!("Export functionality not yet implemented");
    }
    
    fn save_annotated_image(&mut self) {
        // Save current image with overlays
        eprintln!("Save annotated image not yet implemented");
    }
}

impl eframe::App for LeafComplexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Show config editor if requested
        if self.show_config_editor {
            let mut show = self.show_config_editor;
            if self.config_editor.show(ctx, &mut show) {
                // Config was updated
                let new_config = self.config_editor.get_config();
                *self.config.lock().unwrap() = new_config;
            }
            self.show_config_editor = show;
        }
        
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu_bar(ctx, ui);
        });
        
        // Bottom thumbnail strip
        egui::TopBottomPanel::bottom("thumbnails")
            .min_height(120.0)
            .max_height(120.0)
            .show(ctx, |ui| {
                ui::render_thumbnail_strip(ui, &self.state);
            });
        
        // Left panel - Image view
        egui::SidePanel::left("image_view")
            .default_width(600.0)
            .min_width(400.0)
            .max_width(800.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::render_image_view(ui, &self.state, ctx);
            });
        
        // Right panel - Graphs and stats
        egui::CentralPanel::default().show(ctx, |ui| {
            ui::render_analysis_panel(ui, &self.state, ctx);
        });
        
        // Show error toast if present
        if let Some(error) = self.state.lock().unwrap().last_error.clone() {
            egui::Window::new("⚠️ Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(&error);
                    if ui.button("OK").clicked() {
                        self.state.lock().unwrap().last_error = None;
                    }
                });
        }
        
        // Show progress indicator
        {
            let state = self.state.lock().unwrap();
            if state.analysis_in_progress {
                egui::Window::new("⏳ Analyzing...")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.spinner();
                        ui.label("Please wait...");
                    });
            }
        }
    }
}
