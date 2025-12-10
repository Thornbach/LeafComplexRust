// Application State Management
use std::path::PathBuf;
use std::collections::HashMap;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalysisStatus {
    NotStarted,
    Running,
    Completed,
    Failed,
}

#[derive(Clone)]
pub struct AnalysisResult {
    pub ec_data: Vec<(f64, f64)>,  // (Point_Index, Geodesic_EC)
    pub mc_data: Vec<(f64, f64)>,  // (Point_Index, Geodesic_MC_H)
    pub summary: SummaryStats,
    pub ec_image_texture: Option<egui::TextureHandle>,
    pub mc_image_texture: Option<egui::TextureHandle>,
    pub original_texture: Option<egui::TextureHandle>,
}

#[derive(Clone, Default)]
pub struct SummaryStats {
    // EC Stats
    pub ec_length: f64,
    pub ec_width: f64,
    pub ec_shape_index: f64,
    pub ec_circularity: f64,
    pub ec_spectral_entropy: f64,
    
    // MC Stats
    pub mc_length: f64,
    pub mc_width: f64,
    pub mc_shape_index: f64,
    pub mc_spectral_entropy: f64,
    
    // Shared Stats
    pub area: u32,
    pub outline_count: u32,
    pub harmonic_chain_count: usize,
}

pub struct ImageInfo {
    pub path: PathBuf,
    pub filename: String,
    pub thumbnail: Option<egui::TextureHandle>,
    pub status: AnalysisStatus,
}

pub struct AppState {
    // Workspace
    pub workspace_dir: Option<PathBuf>,
    pub images: Vec<ImageInfo>,
    pub current_image_index: Option<usize>,
    
    // Analysis
    pub analysis_results: HashMap<PathBuf, AnalysisResult>,
    pub analysis_in_progress: bool,
    
    // UI State
    pub selected_point: Option<usize>,
    pub show_ec_overlay: bool,
    pub show_mc_overlay: bool,
    pub zoom_level: f32,
    pub pan_offset: egui::Vec2,
    
    // Thumbnail scroll
    pub thumbnail_scroll_offset: f32,
    
    // Error state
    pub last_error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace_dir: None,
            images: Vec::new(),
            current_image_index: None,
            analysis_results: HashMap::new(),
            analysis_in_progress: false,
            selected_point: None,
            show_ec_overlay: true,
            show_mc_overlay: true,
            zoom_level: 1.0,
            pan_offset: egui::Vec2::ZERO,
            thumbnail_scroll_offset: 0.0,
            last_error: None,
        }
    }
}

impl AppState {
    pub fn current_image(&self) -> Option<&ImageInfo> {
        self.current_image_index
            .and_then(|idx| self.images.get(idx))
    }
    
    pub fn current_result(&self) -> Option<&AnalysisResult> {
        self.current_image()
            .and_then(|img| self.analysis_results.get(&img.path))
    }
    
    pub fn load_workspace(&mut self, dir: PathBuf) {
        self.workspace_dir = Some(dir.clone());
        self.images.clear();
        self.current_image_index = None;
        
        // Scan for PNG files
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("png") {
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        self.images.push(ImageInfo {
                            path: path.clone(),
                            filename: filename.to_string(),
                            thumbnail: None,
                            status: AnalysisStatus::NotStarted,
                        });
                    }
                }
            }
        }
        
        // Sort images by filename
        self.images.sort_by(|a, b| a.filename.cmp(&b.filename));
        
        // Select first image if available
        if !self.images.is_empty() {
            self.current_image_index = Some(0);
        }
    }
    
    pub fn select_image(&mut self, index: usize) {
        if index < self.images.len() {
            self.current_image_index = Some(index);
            self.selected_point = None; // Clear point selection
            self.reset_view(); // Reset zoom/pan
        }
    }
    
    pub fn reset_view(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = egui::Vec2::ZERO;
    }
}
