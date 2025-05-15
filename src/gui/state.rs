// src/gui/state.rs - GUI state management and data structures

use std::time::Instant;
use std::collections::HashSet;
use image::RgbaImage;
use minifb::Key;

use crate::config::Config;
use crate::errors::Result;
use crate::feature_extraction::{generate_features, MarginalPointFeatures};
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour};
use crate::path_algorithms::{
    calculate_golden_spiral_params, trace_straight_line, 
    calculate_straight_path_length, check_straight_line_transparency, 
    generate_left_right_spirals, calculate_gyro_path_length,
    calculate_diego_path, calculate_diego_path_length, calculate_diego_path_pink,
    calculate_clr_regions
};
use crate::point_analysis::get_reference_point;

// Constants
pub const WINDOW_WIDTH: usize = 1024;
pub const WINDOW_HEIGHT: usize = 768;
pub const INFO_PANEL_WIDTH: usize = 300;

// Colors (in 0xRRGGBB format)
pub const COLOR_REFERENCE_POINT: u32 = 0xFFFF00; // Yellow
pub const COLOR_CONTOUR_POINT: u32 = 0x00FF00;   // Green
pub const COLOR_SELECTED_POINT: u32 = 0xFF0000;  // Red
pub const COLOR_STRAIGHT_PATH: u32 = 0x0000FF;   // Blue
pub const COLOR_GOLDEN_PATH: u32 = 0xFF8000;     // Orange
pub const COLOR_RIGHT_SPIRAL_PATH: u32 = 0x00FFAA;  // Teal for the right spiral path
pub const COLOR_DIEGO_PATH: u32 = 0x00FFFF;      // Magenta
pub const COLOR_BACKGROUND: u32 = 0x303030;      // Dark gray
pub const COLOR_TEXT: u32 = 0xFFFFFF;            // White
pub const COLOR_SLIDER_BG: u32 = 0x505050;       // Medium gray
pub const COLOR_SLIDER_FG: u32 = 0xD0D0D0;       // Light gray
pub const COLOR_SLIDER_HOVER: u32 = 0xF0F0F0;    // White-ish when hovering
pub const COLOR_CLR_ALPHA: u32 = 0xFF000080;     // Red (semi-transparent) 
pub const COLOR_CLR_GAMMA: u32 = 0x0000FF80;     // Blue (semi-transparent)

// Default ranges
pub const MIN_KERNEL_SIZE: u32 = 1;
pub const MAX_KERNEL_SIZE: u32 = 50;

/// GUI Application State
pub struct GuiState {
    // Input and configuration
    pub config: Config,
    
    // Images
    pub original_image: RgbaImage,
    pub opened_image: Option<RgbaImage>,
    pub marked_image: Option<RgbaImage>,
    
    // Analysis state
    pub kernel_size: u32,
    pub reference_point: Option<(u32, u32)>,
    pub lec_contour: Vec<(u32, u32)>,
    pub selected_point_idx: Option<usize>,
    pub selected_features: Option<MarginalPointFeatures>,
    pub straight_path: Vec<(u32, u32)>,
    pub golden_path: Vec<(u32, u32)>,
    pub left_spiral_path: Vec<(u32, u32)>,
    pub right_spiral_path: Vec<(u32, u32)>,
    pub diego_path: Vec<(u32, u32)>,
    pub clr_alpha_pixels: Vec<(u32, u32)>,
    pub clr_gamma_pixels: Vec<(u32, u32)>,
    pub right_clr_alpha_pixels: Vec<(u32, u32)>,
    pub right_clr_gamma_pixels: Vec<(u32, u32)>,
    
    // Display state
    pub buffer: Vec<u32>,
    pub scale_factor: f32,
    pub offset_x: usize,
    pub offset_y: usize,
    pub display_width: usize,
    
    // UI state
    pub mouse_x: usize,
    pub mouse_y: usize,
    pub mouse_down: bool,
    pub slider_y_coord: usize,
    pub slider_dragging: bool,
    pub last_update: Instant,
    pub status_message: String,
    
    // Debug state
    pub show_transparency: bool,
    pub show_clr_regions: bool,
    pub transparency_check_result: bool,
    pub show_right_spiral: bool,
    pub show_contour_points: bool,
    // Key repeat state for H/L keys
    pub key_repeat_timer: Option<Instant>,
    pub key_repeat_count: u32,
    pub last_key_pressed: Option<Key>,
}

impl GuiState {
    pub fn new(image: RgbaImage, config: Config) -> Self {
        let display_width = WINDOW_WIDTH - INFO_PANEL_WIDTH;
    
        Self {
            config,
            original_image: image,
            opened_image: None,
            marked_image: None,
            kernel_size: 5, // Default
            reference_point: None,
            lec_contour: Vec::new(),
            selected_point_idx: None,
            selected_features: None,
            straight_path: Vec::new(),
            golden_path: Vec::new(),
            left_spiral_path: Vec::new(),
            right_spiral_path: Vec::new(),
            diego_path: Vec::new(),
            clr_alpha_pixels: Vec::new(),
            clr_gamma_pixels: Vec::new(),
            right_clr_alpha_pixels: Vec::new(),
            right_clr_gamma_pixels: Vec::new(),
            buffer: vec![COLOR_BACKGROUND; WINDOW_WIDTH * WINDOW_HEIGHT],
            scale_factor: 1.0,
            offset_x: 0,
            offset_y: 0,
            display_width,
            mouse_x: 0,
            mouse_y: 0,
            mouse_down: false,
            slider_y_coord: 90, // Default slider position
            slider_dragging: false,
            last_update: Instant::now(),
            status_message: String::from("Ready"),
            show_transparency: false,
            show_clr_regions: true,
            transparency_check_result: false,
            show_right_spiral: true, // Start with showing both spirals
            key_repeat_timer: None,
            key_repeat_count: 0,
            last_key_pressed: None,
            show_contour_points: true, // Initialize to true (visible)
        }
    }
    
    /// Update the analysis with current kernel size
    pub fn update_analysis(&mut self) -> Result<()> {
        println!("Updating analysis with kernel size {}", self.kernel_size);
        
        // Apply opening
        self.opened_image = Some(apply_opening(&self.original_image, self.kernel_size)?);
        
        // Mark opened regions
        if let Some(opened) = &self.opened_image {
            self.marked_image = Some(mark_opened_regions(
                &self.original_image,
                opened,
                self.config.marked_region_color_rgb,
            ));
        }
        
        // Calculate reference point
        if let Some(marked) = &self.marked_image {
            println!("Calculating reference point");
            self.reference_point = Some(get_reference_point(
                &self.original_image,
                marked,
                &self.config.reference_point_choice,
                self.config.marked_region_color_rgb,
            )?);
            
            println!("Reference point: {:?}", self.reference_point);
            
            // Trace contour
            println!("Tracing contour");
            self.lec_contour = trace_contour(
                marked,
                true, // is_pink_opaque = true for LEC
                self.config.marked_region_color_rgb,
            );
            
            println!("Found {} contour points", self.lec_contour.len());
            
            // Reset selection
            self.selected_point_idx = None;
            self.selected_features = None;
            self.straight_path.clear();
            self.golden_path.clear();
            self.left_spiral_path.clear();
            self.right_spiral_path.clear();
            self.diego_path.clear();
            self.clr_alpha_pixels.clear();
            self.clr_gamma_pixels.clear();
            self.right_clr_alpha_pixels.clear();
            self.right_clr_gamma_pixels.clear();
        }
        
        // Calculate display scale
        let img_width = self.original_image.width() as usize;
        let img_height = self.original_image.height() as usize;
        
        let scale_x = self.display_width as f32 / img_width as f32;
        let scale_y = WINDOW_HEIGHT as f32 / img_height as f32;
        self.scale_factor = scale_x.min(scale_y).min(1.0);
        
        // Calculate offsets to center the image
        let display_img_width = (img_width as f32 * self.scale_factor) as usize;
        let display_img_height = (img_height as f32 * self.scale_factor) as usize;
        
        self.offset_x = (self.display_width - display_img_width) / 2;
        self.offset_y = (WINDOW_HEIGHT - display_img_height) / 2;
        
        println!("Display dimensions: {}x{} with scale {}", 
                display_img_width, display_img_height, self.scale_factor);
        println!("Offset: {}, {}", self.offset_x, self.offset_y);
        
        Ok(())
    }
    

pub fn select_point(&mut self, idx: usize) -> Result<()> {
    println!("Selecting point {}", idx);
    
    if idx >= self.lec_contour.len() {
        return Ok(());
    }
    
    self.selected_point_idx = Some(idx);
    let marginal_point = self.lec_contour[idx];
    
    // Store marked image reference temporarily
    let marked_image = self.marked_image.as_ref();
    let ref_point = self.reference_point;
    
    if let (Some(marked), Some(ref_point)) = (marked_image, ref_point) {
        // Calculate straight path length (do this early since we'll need it)
        let straight_path_length = calculate_straight_path_length(ref_point, marginal_point);
        
        // Trace straight line path (always calculate this)
        println!("Calculating straight path");
        self.straight_path = trace_straight_line(ref_point, marginal_point);
        
        // Check if straight line crosses transparency
        self.transparency_check_result = check_straight_line_transparency(
            &self.straight_path, 
            marked
        );


        
        println!("Straight line transparency check: {}", self.transparency_check_result);
        
        // Calculate DiegoPath - but if straight line doesn't cross transparency,
        // use exactly the same straight line path to avoid any discrepancies
        println!("Calculating DiegoPath");
        if !self.transparency_check_result {
            // No transparency crossed - use the exact same straight line path object
            // This ensures they are identical and prevents any pixel discrepancies
            self.diego_path = self.straight_path.clone();
        } else {
            // Transparency crossed - calculate proper DiegoPath
            self.diego_path = calculate_diego_path(ref_point, marginal_point, marked);
        }
        
        // Generate features
        println!("Generating features");
        let features = generate_features(
            ref_point,
            &[marginal_point],
            &self.original_image,
            Some(marked),
            self.config.golden_spiral_phi_exponent_factor,
            self.config.marked_region_color_rgb,
            self.config.golden_spiral_rotation_steps,
            true, // is_lec = true
        )?;
        
        if !features.is_empty() {
            self.selected_features = Some(features[0].clone());
            
            // Calculate DiegoPath length and percentage
            // If DiegoPath is identical to the straight path, use straight_path_length directly
            let diego_path_length = if !self.transparency_check_result {
                // When straight path doesn't cross transparency, they should be identical
                straight_path_length
            } else {
                // When transparency is crossed, calculate the length
                calculate_diego_path_length(&self.diego_path)
            };
            
            let diego_path_perc = if straight_path_length > 0.0 {
                (diego_path_length / straight_path_length) * 100.0
            } else {
                100.0 // When they're the same length, percentage is 100%
            };
            
            // Calculate DiegoPath pink if applicable
            let diego_path_pink = if !self.diego_path.is_empty() {
                Some(calculate_diego_path_pink(
                    &self.diego_path, 
                    marked, 
                    self.config.marked_region_color_rgb
                ))
            } else {
                None
            };
            
            // Update the selected_features with the DiegoPath values
            if let Some(features) = &mut self.selected_features {
                features.diego_path_length = diego_path_length;
                features.diego_path_perc = diego_path_perc;
                features.diego_path_pink = diego_path_pink;
            }
            
            if self.transparency_check_result {
                println!("Calculating golden spiral path");
                
                // Calculate spiral parameters
                let (spiral_a_coeff, theta_contact) = 
                    calculate_golden_spiral_params(
                        straight_path_length, 
                        self.config.golden_spiral_phi_exponent_factor
                    );
                
                // Generate both left and right spiral paths
                let (left_path, right_path) = generate_left_right_spirals(
                    ref_point,
                    marginal_point,
                    spiral_a_coeff,
                    theta_contact,
                    self.config.golden_spiral_phi_exponent_factor,
                    (self.config.golden_spiral_rotation_steps * 2) as usize // Higher resolution for visualization
                );
                
                // Store the paths for visualization
                self.golden_path = left_path.clone(); // For backward compatibility
                self.left_spiral_path = left_path;
                self.right_spiral_path = right_path;
                
                // Calculate the actual golden path length using the arc length formula
                let gyro_path_length = calculate_gyro_path_length(
                    spiral_a_coeff,
                    theta_contact,
                    self.config.golden_spiral_phi_exponent_factor
                );
                
                // Update the features with the golden path length
                if let Some(features) = &mut self.selected_features {
                    features.gyro_path_length = gyro_path_length;
                    features.gyro_path_perc = (gyro_path_length / straight_path_length) * 100.0;
                }
                
                println!("Calculated golden path length: {:.2}", gyro_path_length);
                
                // Calculate CLR regions
                let (left_alpha, left_gamma, right_alpha, right_gamma) = if self.show_right_spiral {
                    calculate_clr_regions(
                        ref_point, 
                        marginal_point, 
                        &self.left_spiral_path, 
                        Some(&self.right_spiral_path), 
                        marked
                    )
                } else {
                    calculate_clr_regions(
                        ref_point, 
                        marginal_point, 
                        &self.left_spiral_path, 
                        None, 
                        marked
                    )
                };
                
                // Store the CLR pixel lists for visualization
                self.clr_alpha_pixels = left_alpha;
                self.clr_gamma_pixels = left_gamma;
                self.right_clr_alpha_pixels = right_alpha;
                self.right_clr_gamma_pixels = right_gamma;
                
                // Update the selected features with proper CLR values
                if let Some(features) = &mut self.selected_features {
                    features.left_clr_alpha = self.clr_alpha_pixels.len() as u32;
                    features.left_clr_gamma = self.clr_gamma_pixels.len() as u32;
                    
                    if self.show_right_spiral {
                        features.right_clr_alpha = self.right_clr_alpha_pixels.len() as u32;
                        features.right_clr_gamma = self.right_clr_gamma_pixels.len() as u32;
                        
                        // Calculate averages
                        features.clr_alpha = ((features.left_clr_alpha as f64 + features.right_clr_alpha as f64) / 2.0).floor() as u32;
                        features.clr_gamma = ((features.left_clr_gamma as f64 + features.right_clr_gamma as f64) / 2.0).floor() as u32;
                    } else {
                        // If right spiral not shown, just use left values
                        features.clr_alpha = features.left_clr_alpha;
                        features.clr_gamma = features.left_clr_gamma;
                    }
                }
            } else {
                self.golden_path.clear();
                self.left_spiral_path.clear();
                self.right_spiral_path.clear();
                
                // Reset Gyro path values but keep DiegoPath
                if let Some(features) = &mut self.selected_features {
                    features.gyro_path_length = 0.0;
                    features.gyro_path_perc = 0.0;
                    features.clr_alpha = 0;
                    features.clr_gamma = 0;
                    features.left_clr_alpha = 0;
                    features.left_clr_gamma = 0;
                    features.right_clr_alpha = 0;
                    features.right_clr_gamma = 0;
                }
                
                // Also clear CLR regions
                self.clr_alpha_pixels.clear();
                self.clr_gamma_pixels.clear();
                self.right_clr_alpha_pixels.clear();
                self.right_clr_gamma_pixels.clear();
            }
                            
            println!("Point selection complete");
            self.status_message = format!("Selected point {} at {:?}", idx, marginal_point);
        }
    }
    
    Ok(())
}
    
    /// Find nearest contour point to mouse position
    pub fn find_nearest_contour_point(&self, x: usize, y: usize) -> Option<usize> {
        if self.lec_contour.is_empty() {
            return None;
        }
        
        // Convert screen coordinates to image coordinates
        let img_x = ((x as f32 - self.offset_x as f32) / self.scale_factor) as f32;
        let img_y = ((y as f32 - self.offset_y as f32) / self.scale_factor) as f32;
        
        // Find nearest point
        let mut min_dist = f32::MAX;
        let mut nearest_idx = 0;
        
        for (idx, &(px, py)) in self.lec_contour.iter().enumerate() {
            let dx = img_x - px as f32;
            let dy = img_y - py as f32;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq < min_dist {
                min_dist = dist_sq;
                nearest_idx = idx;
            }
        }
        
        // Only return if within a reasonable distance (400 pixels²)
        if min_dist < 400.0 {
            Some(nearest_idx)
        } else {
            None
        }
    }
    
    /// Check if mouse is on kernel size slider
    pub fn is_mouse_on_slider(&self) -> bool {
        let slider_x = self.display_width + 10;
        let slider_y = self.slider_y_coord;
        let slider_width = INFO_PANEL_WIDTH - 20;
        
        // Make the hit area more generous vertically but centered on the slider
        self.mouse_x >= slider_x && 
        self.mouse_x < slider_x + slider_width && 
        self.mouse_y >= slider_y - 5 && 
        self.mouse_y < slider_y + 5
    }
    
    /// Calculate slider position based on kernel size
    pub fn get_slider_position(&self) -> usize {
        let slider_x = self.display_width + 10;
        let slider_width = INFO_PANEL_WIDTH - 20;
        
        // Map kernel size to slider position
        let pos = ((self.kernel_size - MIN_KERNEL_SIZE) as f32) / 
                  ((MAX_KERNEL_SIZE - MIN_KERNEL_SIZE) as f32);
        
        slider_x + (pos * slider_width as f32) as usize
    }
    
    /// Handle slider movement
    pub fn handle_slider_movement(&mut self) -> Result<()> {
        let slider_x = self.display_width + 10;
        let slider_width = INFO_PANEL_WIDTH - 20;
        
        let pos = (self.mouse_x.saturating_sub(slider_x)) as f32 / slider_width as f32;
        let pos = pos.max(0.0).min(1.0);
        
        // Map position to kernel size (MIN to MAX)
        let new_kernel_size = MIN_KERNEL_SIZE + 
            ((MAX_KERNEL_SIZE - MIN_KERNEL_SIZE) as f32 * pos).round() as u32;
        
        // Clamp to valid range
        let new_kernel_size = new_kernel_size.max(MIN_KERNEL_SIZE).min(MAX_KERNEL_SIZE);
        
        if new_kernel_size != self.kernel_size {
            println!("Changing kernel size from {} to {}", self.kernel_size, new_kernel_size);
            self.kernel_size = new_kernel_size;
            self.update_analysis()?;
        }
        
        Ok(())
    }
    
    /// Handle key repeat for H/L keys
    pub fn handle_key_repeat(&mut self, _key: Key, current_idx: Option<usize>, is_forward: bool) -> Result<()> {
        let contour_len = self.lec_contour.len();
        if contour_len == 0 {
            return Ok(());
        }
        
        let new_idx = if let Some(idx) = current_idx {
            if is_forward {
                // Next point
                if idx < contour_len - 1 {
                    idx + 1
                } else {
                    0 // Wrap to beginning
                }
            } else {
                // Previous point
                if idx > 0 {
                    idx - 1
                } else {
                    contour_len - 1 // Wrap to end
                }
            }
        } else if contour_len > 0 {
            // No current selection, select first or last
            if is_forward {
                0
            } else {
                contour_len - 1
            }
        } else {
            return Ok(());
        };
        
        self.select_point(new_idx)
    }
}