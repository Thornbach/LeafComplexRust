// Main Application Structure
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use std::fs;

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
                        
                        self.generate_all_thumbnails(ctx);
                        ui.close_menu();
                    }
                }
                
                ui.separator();
                
                if ui.button("💾 Export Selected Analysis...").clicked() {
                    self.export_selected_analysis();
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
                ui.checkbox(&mut state.show_path_overlay, "Show Path");
                
                ui.separator();
                
                if ui.button("🔍 Reset Zoom").clicked() {
                    state.reset_view();
                    ui.close_menu();
                }
            });
            
            ui.menu_button("Analysis", |ui| {
                if ui.button("▶ Analyze Current Image").clicked() {
                    self.analyze_current_image(ctx);
                    ui.close_menu();
                }
                
                let selected_count = self.state.lock().unwrap().selected_count();
                let batch_label = if selected_count > 0 {
                    format!("⏩ Analyze Selected ({})", selected_count)
                } else {
                    "⏩ Analyze All Images".to_string()
                };
                
                if ui.button(batch_label).clicked() {
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
                if ui.button("ℹ️ About").clicked() {
                    ui.close_menu();
                }
            });
        });
    }
    
    fn generate_all_thumbnails(&self, ctx: &egui::Context) {
        let state = Arc::clone(&self.state);
        let engine = AnalysisEngine::new();
        let ctx_clone = ctx.clone();
        
        thread::spawn(move || {
            let mut state_guard = state.lock().unwrap();
            let images_clone: Vec<_> = state_guard.images.iter()
                .map(|img| img.path.clone())
                .collect();
            drop(state_guard);
            
            for path in images_clone {
                if let Some(thumbnail) = engine.generate_thumbnail(&path, &ctx_clone) {
                    let mut state_guard = state.lock().unwrap();
                    if let Some(img_info) = state_guard.images.iter_mut().find(|img| img.path == path) {
                        img_info.thumbnail = Some(thumbnail);
                    }
                    drop(state_guard);
                    ctx_clone.request_repaint();
                }
            }
        });
    }
    
    fn analyze_current_image(&mut self, ctx: &egui::Context) {
        let state = Arc::clone(&self.state);
        let config = Arc::clone(&self.config);
        let ctx = ctx.clone();
        
        let image_path = {
            let state_guard = state.lock().unwrap();
            match state_guard.current_image() {
                Some(img) => img.path.clone(),
                None => return,
            }
        };
        
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.analysis_in_progress = true;
            if let Some(idx) = state_guard.current_image_index {
                if let Some(img) = state_guard.images.get_mut(idx) {
                    img.status = AnalysisStatus::Running;
                }
            }
        }
        
        let engine = AnalysisEngine::new();
        thread::spawn(move || {
            let config_guard = config.lock().unwrap();
            let result = engine.analyze_image(&image_path, &config_guard, &ctx);
            drop(config_guard);
            
            let mut state_guard = state.lock().unwrap();
            state_guard.analysis_in_progress = false;
            
            match result {
                Ok(analysis_result) => {
                    state_guard.analysis_results.insert(image_path.clone(), analysis_result);
                    if let Some(idx) = state_guard.current_image_index {
                        if let Some(img) = state_guard.images.get_mut(idx) {
                            img.status = AnalysisStatus::Completed;
                        }
                    }
                }
                Err(e) => {
                    state_guard.last_error = Some(format!("Analysis failed: {}", e));
                    if let Some(idx) = state_guard.current_image_index {
                        if let Some(img) = state_guard.images.get_mut(idx) {
                            img.status = AnalysisStatus::Failed;
                        }
                    }
                }
            }
            
            ctx.request_repaint();
        });
    }
    
    /// FIXED: Work-stealing batch processing - threads pick up new work when finished
    fn analyze_all_images(&mut self, ctx: &egui::Context) {
        let state = Arc::clone(&self.state);
        let config = Arc::clone(&self.config);
        let ctx = ctx.clone();
        
        // Get selected images or all if none selected
        let image_paths: Vec<PathBuf> = {
            let state_guard = state.lock().unwrap();
            let selected = state_guard.get_selected_images();
            if selected.is_empty() {
                // No selection - process all
                state_guard.images.iter().map(|img| img.path.clone()).collect()
            } else {
                selected
            }
        };
        
        if image_paths.is_empty() {
            return;
        }
        
        println!("Starting batch processing of {} images", image_paths.len());
        
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.batch_processing = true;
            state_guard.current_batch_index = 0;
            state_guard.total_batch_count = image_paths.len();
            
            // Mark selected images as running
            for img in state_guard.images.iter_mut() {
                if image_paths.contains(&img.path) {
                    img.status = AnalysisStatus::Running;
                }
            }
        }
        
        // FIXED: Work-stealing queue - threads pick up work dynamically
        thread::spawn(move || {
            use std::sync::mpsc;
            use std::sync::atomic::{AtomicUsize, Ordering};
            
            let num_threads = std::cmp::min(num_cpus::get(), 8);
            println!("Using {} threads for batch processing with work-stealing", num_threads);
            
            // Shared work queue (thread-safe)
            let work_queue = Arc::new(Mutex::new(image_paths.clone()));
            let completed_count = Arc::new(AtomicUsize::new(0));
            let total = image_paths.len();
            
            let (tx, rx) = mpsc::channel();
            let mut handles = vec![];
            
            for thread_id in 0..num_threads {
                let queue = Arc::clone(&work_queue);
                let config = Arc::clone(&config);
                let ctx = ctx.clone();
                let tx = tx.clone();
                let completed = Arc::clone(&completed_count);
                
                let handle = thread::spawn(move || {
                    println!("Thread {} started", thread_id);
                    let engine = AnalysisEngine::new();
                    let config_guard = config.lock().unwrap();
                    
                    let mut processed = 0;
                    loop {
                        // Get next work item from queue
                        let path = {
                            let mut queue_guard = queue.lock().unwrap();
                            queue_guard.pop()
                        };
                        
                        match path {
                            Some(path) => {
                                processed += 1;
                                println!("Thread {}: Processing image {} - {:?}", 
                                        thread_id, processed, path.file_name().unwrap_or_default());
                                
                                let result = engine.analyze_image(&path, &config_guard, &ctx);
                                
                                if tx.send((path.clone(), result)).is_err() {
                                    eprintln!("Thread {}: Failed to send result", thread_id);
                                    break;
                                }
                                
                                completed.fetch_add(1, Ordering::SeqCst);
                            }
                            None => {
                                // No more work
                                println!("Thread {} finished (processed {} images)", thread_id, processed);
                                break;
                            }
                        }
                    }
                    
                    drop(config_guard);
                });
                
                handles.push(handle);
            }
            
            drop(tx);
            
            // Collect results
            let mut completed = 0;
            for (path, result) in rx {
                let mut state_guard = state.lock().unwrap();
                completed += 1;
                state_guard.current_batch_index = completed;
                
                println!("Received result {}/{} for {:?}", 
                        completed, total, path.file_name().unwrap_or_default());
                
                match result {
                    Ok(analysis_result) => {
                        state_guard.analysis_results.insert(path.clone(), analysis_result);
                        if let Some(img) = state_guard.images.iter_mut().find(|i| i.path == path) {
                            img.status = AnalysisStatus::Completed;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to analyze {:?}: {}", path, e);
                        if let Some(img) = state_guard.images.iter_mut().find(|i| i.path == path) {
                            img.status = AnalysisStatus::Failed;
                        }
                    }
                }
                
                drop(state_guard);
                ctx.request_repaint();
            }
            
            for (i, handle) in handles.into_iter().enumerate() {
                if let Err(e) = handle.join() {
                    eprintln!("Thread {} panicked: {:?}", i, e);
                }
            }
            
            let mut state_guard = state.lock().unwrap();
            state_guard.batch_processing = false;
            println!("Batch processing complete! Processed {} images", completed);
        });
    }
    
    /// Export selected images (or all) with proper folder structure
    fn export_selected_analysis(&mut self) {
        let state_guard = self.state.lock().unwrap();
        
        // Get selected images or all if none selected
        let images_to_export: Vec<_> = {
            let selected = state_guard.get_selected_images();
            if selected.is_empty() {
                // No selection - export all analyzed images
                state_guard.images.iter()
                    .filter(|img| state_guard.analysis_results.contains_key(&img.path))
                    .map(|img| (img.filename.clone(), img.path.clone()))
                    .collect()
            } else {
                // Export selected analyzed images
                state_guard.images.iter()
                    .filter(|img| selected.contains(&img.path) && state_guard.analysis_results.contains_key(&img.path))
                    .map(|img| (img.filename.clone(), img.path.clone()))
                    .collect()
            }
        };
        
        if images_to_export.is_empty() {
            drop(state_guard);
            self.state.lock().unwrap().last_error = Some("No analyzed images to export".to_string());
            return;
        }
        
        drop(state_guard);
        
        // Pick export folder
        if let Some(export_base) = rfd::FileDialog::new()
            .set_title("Select Export Location")
            .pick_folder()
        {
            // Create ShapeComplexityResults folder structure
            let results_dir = export_base.join("ShapeComplexityResults");
            let ec_dir = results_dir.join("EC");
            let mc_dir = results_dir.join("MC");
            let summary_dir = results_dir.join("summary");
            
            // Create directories
            if let Err(e) = fs::create_dir_all(&ec_dir) {
                self.state.lock().unwrap().last_error = Some(format!("Failed to create EC directory: {}", e));
                return;
            }
            if let Err(e) = fs::create_dir_all(&mc_dir) {
                self.state.lock().unwrap().last_error = Some(format!("Failed to create MC directory: {}", e));
                return;
            }
            if let Err(e) = fs::create_dir_all(&summary_dir) {
                self.state.lock().unwrap().last_error = Some(format!("Failed to create summary directory: {}", e));
                return;
            }
            
            println!("Exporting {} images to {:?}", images_to_export.len(), results_dir);
            
            // Collect all summaries for the single summary CSV
            let mut all_summaries = Vec::new();
            
            for (filename, path) in &images_to_export {
                let state_guard = self.state.lock().unwrap();
                if let Some(result) = state_guard.analysis_results.get(path) {
                    let result_clone = result.clone();
                    drop(state_guard);
                    
                    // Export EC data
                    let ec_path = ec_dir.join(format!("{}_EC.csv", filename));
                    if let Err(e) = self.write_csv(&ec_path, &result_clone.ec_data, "Point_Index,Pink_Pixels") {
                        eprintln!("Failed to export EC for {}: {}", filename, e);
                        continue;
                    }
                    
                    // Export MC data
                    let mc_path = mc_dir.join(format!("{}_MC.csv", filename));
                    if let Err(e) = self.write_csv(&mc_path, &result_clone.mc_data, "Point_Index,Geodesic_MC_H") {
                        eprintln!("Failed to export MC for {}: {}", filename, e);
                        continue;
                    }
                    
                    // Collect summary
                    all_summaries.push((filename.clone(), result_clone.summary));
                    
                    println!("Exported: {}", filename);
                } else {
                    drop(state_guard);
                }
            }
            
            // Write single summary CSV with all images
            let summary_path = summary_dir.join("summary.csv");
            if let Err(e) = self.write_multi_summary_csv(&summary_path, &all_summaries) {
                self.state.lock().unwrap().last_error = Some(format!("Failed to write summary: {}", e));
                return;
            }
            
            println!("Export complete! {} images exported to {:?}", images_to_export.len(), results_dir);
        }
    }
    
    fn write_csv(&self, path: &PathBuf, data: &[(f64, f64)], header: &str) -> Result<(), String> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        writeln!(file, "{}", header)
            .map_err(|e| format!("Failed to write header: {}", e))?;
        
        for (x, y) in data {
            writeln!(file, "{},{}", x, y)
                .map_err(|e| format!("Failed to write data: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Write summary CSV with multiple images (one row per image)
    fn write_multi_summary_csv(&self, path: &PathBuf, summaries: &[(String, crate::state::SummaryStats)]) -> Result<(), String> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        // Write header
        writeln!(
            file,
            "ID,MC,EC,EC_Length,MC_Length,EC_Width,MC_Width,EC_ShapeIndex,MC_ShapeIndex,EC_Circularity,MC_Circularity,EC_Area,MC_Area,EC_Outline_Count,MC_Outline_Count"
        ).map_err(|e| format!("Failed to write header: {}", e))?;
        
        // Write each image as a row
        for (filename, summary) in summaries {
            writeln!(
                file,
                "{},{:.4},{:.4},{:.1},{:.1},{:.1},{:.1},{:.3},{:.3},{:.3},{:.3},{},{},{},{}",
                filename,
                summary.mc_spectral_entropy,
                summary.ec_spectral_entropy,
                summary.ec_length,
                summary.mc_length,
                summary.ec_width,
                summary.mc_width,
                summary.ec_shape_index,
                summary.mc_shape_index,
                summary.ec_circularity,
                summary.mc_circularity,
                summary.ec_area,
                summary.mc_area,
                summary.ec_outline_count,
                summary.mc_outline_count
            ).map_err(|e| format!("Failed to write data: {}", e))?;
        }
        
        Ok(())
    }
}

impl eframe::App for LeafComplexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.show_config_editor {
            let mut show = self.show_config_editor;
            let config_updated = self.config_editor.show(ctx, &mut show);
            self.show_config_editor = show;  // Update immediately
            
            if config_updated {
                let new_config = self.config_editor.get_config();
                *self.config.lock().unwrap() = new_config;
                println!("✅ Configuration updated and applied!");
            }
        }
        
        let mut analyze_clicked = false;
        let mut batch_clicked = false;
        
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu_bar(ctx, ui);
        });
        
        egui::TopBottomPanel::bottom("thumbnails")
            .min_height(150.0)  // Increased for checkboxes
            .max_height(150.0)
            .show(ctx, |ui| {
                ui::render_thumbnail_strip(ui, &self.state);
            });
        
        egui::SidePanel::left("image_view")
            .default_width(600.0)
            .min_width(400.0)
            .max_width(800.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui::render_image_view(ui, &self.state, ctx, &mut analyze_clicked, &mut batch_clicked);
            });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui::render_analysis_panel(ui, &self.state, ctx);
        });
        
        if analyze_clicked {
            self.analyze_current_image(ctx);
        }
        if batch_clicked {
            self.analyze_all_images(ctx);
        }
        
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
            
            if state.batch_processing {
                egui::Window::new("⏳ Batch Processing...")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.spinner();
                        ui.label(format!("Processing {} of {}", 
                            state.current_batch_index, 
                            state.total_batch_count));
                        
                        let progress = if state.total_batch_count > 0 {
                            state.current_batch_index as f32 / state.total_batch_count as f32
                        } else {
                            0.0
                        };
                        ui.add(egui::ProgressBar::new(progress).show_percentage());
                    });
            }
        }
    }
}
