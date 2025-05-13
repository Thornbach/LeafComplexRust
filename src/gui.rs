use image::RgbaImage;
use minifb::{Key, Window, WindowOptions};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::{generate_features, MarginalPointFeatures};
use crate::font::FONT_BITMAP;
use crate::image_io::load_image;
use crate::image_utils::resize_image;
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour};
use crate::path_algorithms::{
    calculate_golden_spiral_params, trace_straight_line, 
    calculate_straight_path_length, check_straight_line_transparency, 
    is_point_in_polygon, calculate_gyro_path_length,
    generate_left_right_spirals, calculate_clr_points, calculate_gyro_path_pink,
    calculate_diego_path, calculate_diego_path_length, calculate_diego_path_pink
};
use crate::point_analysis::get_reference_point;

// Constants
const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 768;
const INFO_PANEL_WIDTH: usize = 300;

// Colors (in 0xRRGGBB format)
const COLOR_REFERENCE_POINT: u32 = 0xFFFF00; // Yellow
const COLOR_CONTOUR_POINT: u32 = 0x00FF00;   // Green
const COLOR_SELECTED_POINT: u32 = 0xFF0000;  // Red
const COLOR_STRAIGHT_PATH: u32 = 0x0000FF;   // Blue
const COLOR_GOLDEN_PATH: u32 = 0xFF8000;     // Orange
const COLOR_RIGHT_SPIRAL_PATH: u32 = 0x00FFAA;  // Teal for the right spiral path
const COLOR_BACKGROUND: u32 = 0x303030;      // Dark gray
const COLOR_TEXT: u32 = 0xFFFFFF;            // White
const COLOR_SLIDER_BG: u32 = 0x505050;       // Medium gray
const COLOR_SLIDER_FG: u32 = 0xD0D0D0;       // Light gray
const COLOR_SLIDER_HOVER: u32 = 0xF0F0F0;    // White-ish when hovering
const COLOR_CLR_ALPHA: u32 = 0xFF000080;     // Red (semi-transparent) 
const COLOR_CLR_GAMMA: u32 = 0x0000FF80;     // Blue (semi-transparent)

// Default ranges
const MIN_KERNEL_SIZE: u32 = 1;
const MAX_KERNEL_SIZE: u32 = 50;

//  ██████  ██    ██ ██     ███████ ████████ ██████  ██    ██  ██████ ████████ 
// ██       ██    ██ ██     ██         ██    ██   ██ ██    ██ ██         ██    
// ██   ███ ██    ██ ██     ███████    ██    ██████  ██    ██ ██         ██    
// ██    ██ ██    ██ ██          ██    ██    ██   ██ ██    ██ ██         ██    
//  ██████   ██████  ██     ███████    ██    ██   ██  ██████   ██████    ██    

/// GUI Application State
struct GuiState {
    // Input and configuration
    config: Config,
    
    // Images
    original_image: RgbaImage,
    opened_image: Option<RgbaImage>,
    marked_image: Option<RgbaImage>,
    
    // Analysis state
    kernel_size: u32,
    reference_point: Option<(u32, u32)>,
    lec_contour: Vec<(u32, u32)>,
    selected_point_idx: Option<usize>,
    selected_features: Option<MarginalPointFeatures>,
    straight_path: Vec<(u32, u32)>,
    golden_path: Vec<(u32, u32)>,
    left_spiral_path: Vec<(u32, u32)>,
    right_spiral_path: Vec<(u32, u32)>,
    diego_path: Vec<(u32, u32)>, // New: DiegoPath
    clr_alpha_pixels: Vec<(u32, u32)>,
    clr_gamma_pixels: Vec<(u32, u32)>,
    right_clr_alpha_pixels: Vec<(u32, u32)>,
    right_clr_gamma_pixels: Vec<(u32, u32)>,
    
    // Display state
    buffer: Vec<u32>,
    scale_factor: f32,
    offset_x: usize,
    offset_y: usize,
    display_width: usize,
    
    // UI state
    mouse_x: usize,
    mouse_y: usize,
    mouse_down: bool,
    slider_y_coord: usize,
    slider_dragging: bool,
    last_update: Instant,
    status_message: String,
    
    // Debug state
    show_transparency: bool,
    show_clr_regions: bool,
    transparency_check_result: bool,
    show_right_spiral: bool,
    
    // Key repeat state for H/L keys
    key_repeat_timer: Option<Instant>,
    key_repeat_count: u32,
    last_key_pressed: Option<Key>,
}

impl GuiState {
    fn new(image: RgbaImage, config: Config) -> Self {
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
            golden_path: Vec::new(), // This is now the left path
            left_spiral_path: Vec::new(),
            right_spiral_path: Vec::new(),
            diego_path: Vec::new(), // Initialize DiegoPath
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
        }
    }
    
    /// Update the analysis with current kernel size
    fn update_analysis(&mut self) -> Result<()> {
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
            self.diego_path.clear(); // Clear DiegoPath
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
    
    /// Select a point on the contour
    fn select_point(&mut self, idx: usize) -> Result<()> {
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
                
                // Calculate straight path
                println!("Calculating straight path");
                self.straight_path = trace_straight_line(ref_point, marginal_point);
                
                // Calculate DiegoPath (always calculate)
                println!("Calculating DiegoPath");
                self.diego_path = calculate_diego_path(ref_point, marginal_point, marked);
                
                // Check if straight line crosses transparency
                self.transparency_check_result = check_straight_line_transparency(
                    &self.straight_path, 
                    marked
                );
                
                println!("Straight line transparency check: {}", self.transparency_check_result);
                
                if self.transparency_check_result {
                    println!("Calculating golden spiral path");
                    
                    // Get the straight path length
                    let straight_path_length = calculate_straight_path_length(ref_point, marginal_point);
                    
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
                    
                    // Calculate CLR values for both paths
                    let (left_alpha, left_gamma) = calculate_clr_points(ref_point, marginal_point, &self.left_spiral_path, marked);
                    let (right_alpha, right_gamma) = calculate_clr_points(ref_point, marginal_point, &self.right_spiral_path, marked);
                    
                    // Update the selected_features with the new CLR values
                    if let Some(features) = &mut self.selected_features {
                        // Store individual values
                        features.left_clr_alpha = left_alpha;
                        features.left_clr_gamma = left_gamma;
                        features.right_clr_alpha = right_alpha;
                        features.right_clr_gamma = right_gamma;
                        
                        // Calculate averages
                        features.clr_alpha = ((left_alpha as f64 + right_alpha as f64) / 2.0).floor() as u32;
                        features.clr_gamma = ((left_gamma as f64 + right_gamma as f64) / 2.0).floor() as u32;
                    }
                    
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
                
                // Calculate DiegoPath length and percentage, updating the selected features
                if !self.diego_path.is_empty() {
                    if let Some(features) = &mut self.selected_features {
                        let straight_path_length = calculate_straight_path_length(
                            self.reference_point.unwrap_or((0, 0)),
                            self.lec_contour[self.selected_point_idx.unwrap_or(0)]
                        );
                        features.diego_path_length = calculate_diego_path_length(&self.diego_path);
                        features.diego_path_perc = (features.diego_path_length / straight_path_length) * 100.0;
                        
                        // Calculate DiegoPath pink if in LEC mode
                        features.diego_path_pink = Some(calculate_diego_path_pink(
                            &self.diego_path, 
                            marked, 
                            self.config.marked_region_color_rgb
                        ));
                    }
                }
                                
                println!("Point selection complete");
                self.status_message = format!("Selected point {} at {:?}", idx, marginal_point);
            }
        }
        
        // Calculate CLR regions after the if let block
        if !self.golden_path.is_empty() {
            // Clone the necessary data to avoid borrow conflicts
            let marked = self.marked_image.as_ref().map(|img| img.clone());
            let ref_point = self.reference_point;
            
            if let (Some(marked), Some(ref_point)) = (marked, ref_point) {
                self.calculate_clr_regions(ref_point, marginal_point, &marked);
                
                // Update the selected_features with the new CLR values
                if let Some(features) = &mut self.selected_features {
                    features.clr_alpha = self.clr_alpha_pixels.len() as u32;
                    features.clr_gamma = self.clr_gamma_pixels.len() as u32;
                    // Make sure right CLR values are properly updated
                    features.right_clr_alpha = self.right_clr_alpha_pixels.len() as u32;
                    features.right_clr_gamma = self.right_clr_gamma_pixels.len() as u32;
                }
            }
        } else {
            // Always clear all CLR regions when no gyro path is found
            self.clr_alpha_pixels.clear();
            self.clr_gamma_pixels.clear();
            self.right_clr_alpha_pixels.clear();
            self.right_clr_gamma_pixels.clear();
            
            // Reset CLR values in selected_features
            if let Some(features) = &mut self.selected_features {
                features.clr_alpha = 0;
                features.clr_gamma = 0;
                features.left_clr_alpha = 0;
                features.left_clr_gamma = 0;
                features.right_clr_alpha = 0;
                features.right_clr_gamma = 0;
            }
        }
        
        Ok(())
    }
    /// Find nearest contour point to mouse position
    fn find_nearest_contour_point(&self, x: usize, y: usize) -> Option<usize> {
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
    fn is_mouse_on_slider(&self) -> bool {
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
    fn get_slider_position(&self) -> usize {
        let slider_x = self.display_width + 10;
        let slider_width = INFO_PANEL_WIDTH - 20;
        
        // Map kernel size to slider position
        let pos = ((self.kernel_size - MIN_KERNEL_SIZE) as f32) / 
                  ((MAX_KERNEL_SIZE - MIN_KERNEL_SIZE) as f32);
        
        slider_x + (pos * slider_width as f32) as usize
    }
    
    /// Handle slider movement
    fn handle_slider_movement(&mut self) -> Result<()> {
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
    
    /// Calculate CLR regions
    fn calculate_clr_regions(&mut self, ref_point: (u32, u32), margin_point: (u32, u32), image: &RgbaImage) {
        println!("Calculating CLR regions");
        self.clr_alpha_pixels.clear();
        self.clr_gamma_pixels.clear();
        self.right_clr_alpha_pixels.clear();
        self.right_clr_gamma_pixels.clear();
        
        // Create polygon from straight line and golden path
        let mut polygon = Vec::new();
        polygon.extend_from_slice(&self.straight_path);
        
        // Reverse golden path for proper polygon formation
        let mut golden_path_rev = self.golden_path.clone();
        golden_path_rev.reverse();
        polygon.extend_from_slice(&golden_path_rev);
        
        // Calculate bounding box (with padding)
        let padding = 10;
        let min_x = ref_point.0.min(margin_point.0).saturating_sub(padding);
        let max_x = ref_point.0.max(margin_point.0) + padding;
        let min_y = ref_point.1.min(margin_point.1).saturating_sub(padding);
        let max_y = ref_point.1.max(margin_point.1) + padding;
        
        // Expand bounding box to include golden path
        let expanded_bbox = self.golden_path.iter().fold((min_x, min_y, max_x, max_y), |acc, &(x, y)| {
            (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y))
        });
        
        let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = expanded_bbox;
        
        // Count pixels in each category
        let width = image.width();
        let height = image.height();
        
        for y in bbox_min_y..=bbox_max_y {
            if y >= height {
                continue;
            }
            
            for x in bbox_min_x..=bbox_max_x {
                if x >= width {
                    continue;
                }
                
                // Check if the point is inside the polygon
                if is_point_in_polygon(x as f32, y as f32, &polygon) {
                    let pixel = image.get_pixel(x, y);
                    
                    // Check if transparent
                    if pixel[3] == 0 {
                        self.clr_alpha_pixels.push((x, y));
                    } else {
                        self.clr_gamma_pixels.push((x, y));
                    }
                }
            }
        }
        
        // Also calculate for right spiral if enabled
        if self.show_right_spiral && !self.right_spiral_path.is_empty() {
            // Similar process for right spiral
            let mut right_polygon = Vec::new();
            right_polygon.extend_from_slice(&self.straight_path);
            
            let mut right_path_rev = self.right_spiral_path.clone();
            right_path_rev.reverse();
            right_polygon.extend_from_slice(&right_path_rev);
            
            // Use the same bounding box expanded to include right spiral path
            let right_expanded_bbox = self.right_spiral_path.iter().fold(
                (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y), 
                |acc, &(x, y)| {
                    (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y))
                }
            );
            
            let (r_bbox_min_x, r_bbox_min_y, r_bbox_max_x, r_bbox_max_y) = right_expanded_bbox;
            
            for y in r_bbox_min_y..=r_bbox_max_y {
                if y >= height {
                    continue;
                }
                
                for x in r_bbox_min_x..=r_bbox_max_x {
                    if x >= width {
                        continue;
                    }
                    
                    if is_point_in_polygon(x as f32, y as f32, &right_polygon) {
                        let pixel = image.get_pixel(x, y);
                        
                        if pixel[3] == 0 {
                            self.right_clr_alpha_pixels.push((x, y));
                        } else {
                            self.right_clr_gamma_pixels.push((x, y));
                        }
                    }
                }
            }
        }
        
        println!("CLR_Alpha: {}, CLR_Gamma: {}", 
                self.clr_alpha_pixels.len(), self.clr_gamma_pixels.len());
                
        if self.show_right_spiral {
            println!("Right CLR_Alpha: {}, Right CLR_Gamma: {}", 
                    self.right_clr_alpha_pixels.len(), self.right_clr_gamma_pixels.len());
        }
    }


    // ██████  ██    ██ ███████ ███████ ███████ ██████  
    // ██   ██ ██    ██ ██      ██      ██      ██   ██ 
    // ██████  ██    ██ █████   █████   █████   ██████  
    // ██   ██ ██    ██ ██      ██      ██      ██   ██ 
    // ██████   ██████  ██      ██      ███████ ██   ██ 
                                                     
                                                    
    
    /// Update the buffer for display
    fn update_buffer(&mut self) {
        // Add a new color for DiegoPath
        const COLOR_DIEGO_PATH: u32 = 0xFF00FF; // Magenta
    
        // Clear buffer
        for pixel in &mut self.buffer {
            *pixel = COLOR_BACKGROUND;
        }
        
        // Draw image
        if let Some(img) = &self.marked_image {
            let img_width = img.width() as usize;
            let img_height = img.height() as usize;
            
            // Draw the image
            for y in 0..img_height {
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                if display_y >= WINDOW_HEIGHT {
                    continue;
                }
                
                for x in 0..img_width {
                    let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                    if display_x >= self.display_width {
                        continue;
                    }
                    
                    let pixel = img.get_pixel(x as u32, y as u32);
                    
                    // Handle transparency visualization
                    if self.show_transparency && pixel[3] == 0 {
                        // Show transparent pixels as a checkerboard pattern
                        let checker = (x + y) % 2 == 0;
                        let color = if checker { 0x606060 } else { 0x404040 };
                        
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < self.buffer.len() {
                            self.buffer[idx] = color;
                        }
                    }
                    // Skip fully transparent pixels unless showing transparency
                    else if pixel[3] > 0 {
                        let r = pixel[0] as u32;
                        let g = pixel[1] as u32;
                        let b = pixel[2] as u32;
                        let color = (r << 16) | (g << 8) | b;
                        
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < self.buffer.len() {
                            self.buffer[idx] = color;
                        }
                    }
                }
            }
            
            // Draw CLR regions if enabled
            if self.show_clr_regions {
                // Draw CLR_Alpha (transparent) pixels
                for (i, &(x, y)) in self.clr_alpha_pixels.iter().enumerate() {
                    // Sample every few pixels for performance
                    if i % 4 != 0 {
                        continue;
                    }
                    
                    let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                    let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                    
                    if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < self.buffer.len() {
                            self.buffer[idx] = COLOR_CLR_ALPHA;
                        }
                    }
                }
                
                // Draw CLR_Gamma (non-transparent) pixels
                for (i, &(x, y)) in self.clr_gamma_pixels.iter().enumerate() {
                    // Sample every few pixels for performance
                    if i % 4 != 0 {
                        continue;
                    }
                    
                    let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                    let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                    
                    if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < self.buffer.len() {
                            self.buffer[idx] = COLOR_CLR_GAMMA;
                        }
                    }
                }
    
                if self.show_right_spiral && !self.right_clr_alpha_pixels.is_empty() {
                    // Draw right spiral CLR regions with different colors
                    for (i, &(x, y)) in self.right_clr_alpha_pixels.iter().enumerate() {
                        // Sample every few pixels for performance
                        if i % 4 != 0 {
                            continue;
                        }
                        
                        let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                        let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                        
                        if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                            let idx = display_y * WINDOW_WIDTH + display_x;
                            if idx < self.buffer.len() {
                                // Use a slightly different color for right spiral
                                self.buffer[idx] = 0xFF0000A0; // More reddish
                            }
                        }
                    }
    
                    if !self.right_clr_gamma_pixels.is_empty() {
                        // Draw right spiral CLR gamma pixels
                        for (i, &(x, y)) in self.right_clr_gamma_pixels.iter().enumerate() {
                            // Sample every few pixels for performance
                            if i % 4 != 0 {
                                continue;
                            }
                            
                            let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                            let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                            
                            if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                                let idx = display_y * WINDOW_WIDTH + display_x;
                                if idx < self.buffer.len() {
                                    // Use a slightly different color for right spiral
                                    self.buffer[idx] = 0xFF8000A0; // More orangeish
                                }
                            }
                        }
                    }
                }
            }
            
            // Draw paths
            
            // Draw straight path
            for &(x, y) in &self.straight_path {
                let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                
                if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < self.buffer.len() {
                        self.buffer[idx] = COLOR_STRAIGHT_PATH;
                    }
                }
            }
            
            // Draw DiegoPath (always draw if available)
            for &(x, y) in &self.diego_path {
                let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                
                if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < self.buffer.len() {
                        self.buffer[idx] = COLOR_DIEGO_PATH;
                    }
                }
            }
            
            // Draw golden path
            for &(x, y) in &self.left_spiral_path {
                let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                
                if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < self.buffer.len() {
                        self.buffer[idx] = COLOR_GOLDEN_PATH; // Keep the original color for backward compatibility
                    }
                }
            }
    
            // Draw right spiral path if enabled
            if self.show_right_spiral {
                for &(x, y) in &self.right_spiral_path {
                    let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                    let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                    
                    if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < self.buffer.len() {
                            self.buffer[idx] = COLOR_RIGHT_SPIRAL_PATH;
                        }
                    }
                }
            }
            
            // Draw contour points
            for (i, &point) in self.lec_contour.iter().enumerate() {
                let display_x = (point.0 as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (point.1 as f32 * self.scale_factor) as usize + self.offset_y;
                
                if display_x < self.display_width && display_y < WINDOW_HEIGHT {
                    draw_circle(&mut self.buffer, display_x, display_y, 1, 
                        WINDOW_WIDTH, WINDOW_HEIGHT,
                        if Some(i) == self.selected_point_idx {
                            COLOR_SELECTED_POINT
                        } else {
                            COLOR_CONTOUR_POINT
                        });
                }
            }
            
            // Draw selected point
            if let Some(idx) = self.selected_point_idx {
                let (x, y) = self.lec_contour[idx];
                let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                
                draw_circle(&mut self.buffer, display_x, display_y, 4, 
                    WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_SELECTED_POINT);
            }
            
            // Draw reference point
            if let Some((x, y)) = self.reference_point {
                let display_x = (x as f32 * self.scale_factor) as usize + self.offset_x;
                let display_y = (y as f32 * self.scale_factor) as usize + self.offset_y;
                
                draw_circle(&mut self.buffer, display_x, display_y, 5, 
                    WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_REFERENCE_POINT);
            }
        }
        
        // Draw info panel background
        let info_panel_x = self.display_width;
        for y in 0..WINDOW_HEIGHT {
            for x in info_panel_x..WINDOW_WIDTH {
                let idx = y * WINDOW_WIDTH + x;
                self.buffer[idx] = 0x202020; // Dark gray
            }
        }
        
        // Draw info panel separator
        for y in 0..WINDOW_HEIGHT {
            let idx = y * WINDOW_WIDTH + info_panel_x;
            self.buffer[idx] = 0x505050; // Medium gray
        }
        
        // Draw text in info panel
        let panel_x = self.display_width + 10;
        let mut text_y = 20;
        
        // Title
        draw_text_bitmap(&mut self.buffer, "LeafComplexR Visualizer", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 30;
        
        // Kernel size
        draw_text_bitmap(&mut self.buffer, &format!("Kernel Size: {}", self.kernel_size), 
                panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Store the Y coordinate for the slider and draw it
        self.slider_y_coord = text_y;
    
        // Draw a slider for kernel size
        let slider_x = panel_x;
        let slider_width = INFO_PANEL_WIDTH - 20;
        let slider_handle_pos = self.get_slider_position();
        
        // Slider track
        for x_pos in slider_x..(slider_x + slider_width) {
            let idx = self.slider_y_coord * WINDOW_WIDTH + x_pos;
            if idx < self.buffer.len() {
                self.buffer[idx] = COLOR_SLIDER_BG;
            }
        }
        
        // Slider handle
        let handle_color = if self.is_mouse_on_slider() || self.slider_dragging {
            COLOR_SLIDER_HOVER
        } else {
            COLOR_SLIDER_FG
        };
        
        draw_circle(&mut self.buffer, slider_handle_pos, self.slider_y_coord, 5, 
                   WINDOW_WIDTH, WINDOW_HEIGHT, handle_color);
        
        text_y += 30;
        
        // ██      ███████  ██████  ███████ ███    ██ ██████  
        // ██      ██      ██       ██      ████   ██ ██   ██ 
        // ██      █████   ██   ███ █████   ██ ██  ██ ██   ██ 
        // ██      ██      ██    ██ ██      ██  ██ ██ ██   ██ 
        // ███████ ███████  ██████  ███████ ██   ████ ██████  

        // Reference point
        if let Some(point) = self.reference_point {
            draw_text_bitmap(&mut self.buffer, &format!("Reference Point: ({}, {})", point.0, point.1), 
                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        }
        text_y += 20;
        
        // Transparency check
        if self.selected_point_idx.is_some() {
            let result_str = if self.transparency_check_result { "YES" } else { "NO" };
            draw_text_bitmap(&mut self.buffer, &format!("Crosses transparency: {}", result_str), 
                     panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
        }
        
        if let (Some(idx), Some(features)) = (self.selected_point_idx, &self.selected_features) {
            let point = self.lec_contour[idx];
            
            text_y += 10;
            draw_text_bitmap(&mut self.buffer, "Selected Point Data:", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            draw_text_bitmap(&mut self.buffer, &format!("Point ({}, {}) [idx: {}]", point.0, point.1, idx), 
                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            draw_text_bitmap(&mut self.buffer, &format!("StraightPath: {:.2}", features.straight_path_length), 
                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            // Always show DiegoPath info - this was missing/incomplete before
            draw_text_bitmap(&mut self.buffer, &format!("DiegoPath: {:.2}", features.diego_path_length), 
                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            draw_text_bitmap(&mut self.buffer, &format!("DiegoPath %: {:.2}", features.diego_path_perc), 
                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            if let Some(diego_pink) = features.diego_path_pink {
                draw_text_bitmap(&mut self.buffer, &format!("DiegoPath_Pink: {}", diego_pink), 
                        panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
            }
            
            if features.gyro_path_length > 0.0 {
                draw_text_bitmap(&mut self.buffer, &format!("GyroPath: {:.2}", features.gyro_path_length), 
                        panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
                
                // Display averaged CLR values
                draw_text_bitmap(&mut self.buffer, &format!("Avg CLR_Alpha: {}", features.clr_alpha), 
                        panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
                
                draw_text_bitmap(&mut self.buffer, &format!("Avg CLR_Gamma: {}", features.clr_gamma), 
                        panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
                
                // Display individual CLR values if right spiral is shown
                draw_text_bitmap(&mut self.buffer, &format!("Left CLR_Alpha: {}", features.left_clr_alpha), 
                                panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
                
                draw_text_bitmap(&mut self.buffer, &format!("Left CLR_Gamma: {}", features.left_clr_gamma), 
                                panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                text_y += 20;
                
                if self.show_right_spiral {
                    draw_text_bitmap(&mut self.buffer, &format!("Right CLR_Alpha: {}", features.right_clr_alpha), 
                                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                    text_y += 20;
                    
                    draw_text_bitmap(&mut self.buffer, &format!("Right CLR_Gamma: {}", features.right_clr_gamma), 
                                    panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
                    text_y += 20;
                }
            }
        }
        
        // Legend
        text_y += 20;
        draw_text_bitmap(&mut self.buffer, "Legend:", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Use color boxes with text
        let color_box_size = 10;
        let color_text_gap = 5;
        
        // Reference Point
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_REFERENCE_POINT);
        draw_text_bitmap(&mut self.buffer, "Reference Point", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Contour Points
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CONTOUR_POINT);
        draw_text_bitmap(&mut self.buffer, "Contour Points", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Selected Point
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_SELECTED_POINT);
        draw_text_bitmap(&mut self.buffer, "Selected Point", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Straight Path
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_STRAIGHT_PATH);
        draw_text_bitmap(&mut self.buffer, "Straight Path", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Diego Path
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_DIEGO_PATH);
        draw_text_bitmap(&mut self.buffer, "Diego Path (Within Leaf)", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Golden Path
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_GOLDEN_PATH);
        draw_text_bitmap(&mut self.buffer, "Left Golden Spiral Path", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        // Right Spiral Path
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
            WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_RIGHT_SPIRAL_PATH);
        draw_text_bitmap(&mut self.buffer, "Right Spiral Path", 
        panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // CLR Alpha
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CLR_ALPHA);
        draw_text_bitmap(&mut self.buffer, "CLR_Alpha (Transparent)", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // CLR Gamma
        draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                 WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CLR_GAMMA);
        draw_text_bitmap(&mut self.buffer, "CLR_Gamma (Non-Transparent)", 
                panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
        
        // Right CLR Alpha (only show if right spiral is toggled)
        if self.show_right_spiral {
            draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                    WINDOW_WIDTH, WINDOW_HEIGHT, 0xFF0000A0);
            draw_text_bitmap(&mut self.buffer, "Right CLR_Alpha", 
                    panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
            
            // Right CLR Gamma
            draw_rect(&mut self.buffer, panel_x, text_y, color_box_size, color_box_size, 
                    WINDOW_WIDTH, WINDOW_HEIGHT, 0xFF8000A0);
            draw_text_bitmap(&mut self.buffer, "Right CLR_Gamma", 
                    panel_x + color_box_size + color_text_gap, text_y, WINDOW_WIDTH, COLOR_TEXT);
            text_y += 20;
        }
        
        text_y += 5;
        
        // Controls
        draw_text_bitmap(&mut self.buffer, "Controls:", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- Click: Select contour point", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- H/L: Previous/Next point", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- R: Toggle right spiral path", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- T: Toggle transparency view", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- C: Toggle CLR regions", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    
        draw_text_bitmap(&mut self.buffer, "- Esc: Exit", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        // Don't increment text_y anymore since it's not used after this
        
        // Status message at bottom
        let status_y = WINDOW_HEIGHT - 20;
        draw_text_bitmap(&mut self.buffer, &self.status_message, panel_x, status_y, WINDOW_WIDTH, COLOR_TEXT);
    }

    fn handle_key_repeat(&mut self, key: Key, current_idx: Option<usize>, is_forward: bool) -> Result<()> {
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
/// Draw a circle
fn draw_circle(buffer: &mut [u32], center_x: usize, center_y: usize, radius: usize, 
               width: usize, height: usize, color: u32) {
    let radius_sq = (radius * radius) as isize;
    
    for y in center_y.saturating_sub(radius)..std::cmp::min(center_y + radius + 1, height) {
        for x in center_x.saturating_sub(radius)..std::cmp::min(center_x + radius + 1, width) {
            let dx = x as isize - center_x as isize;
            let dy = y as isize - center_y as isize;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq <= radius_sq {
                let idx = y * width + x;
                if idx < buffer.len() {
                    buffer[idx] = color;
                }
            }
        }
    }
}

// ██████  ██████   █████  ██     ██ 
// ██   ██ ██   ██ ██   ██ ██     ██ 
// ██   ██ ██████  ███████ ██  █  ██ 
// ██   ██ ██   ██ ██   ██ ██ ███ ██ 
// ██████  ██   ██ ██   ██  ███ ███  
                                 

/// Draw a rectangle
fn draw_rect(buffer: &mut [u32], x: usize, y: usize, width_px: usize, height_px: usize,
             buffer_width: usize, buffer_height: usize, color: u32) {
    for py in y..std::cmp::min(y + height_px, buffer_height) {
        for px in x..std::cmp::min(x + width_px, buffer_width) {
            let idx = py * buffer_width + px;
            if idx < buffer.len() {
                buffer[idx] = color;
            }
        }
    }
}

/// Draw text using the bitmap font
fn draw_text_bitmap(buffer: &mut [u32], text: &str, x: usize, y: usize, width: usize, color: u32) {
    let mut cursor_x = x;
    
    for c in text.chars() {
        // Check if the character is in the range of our font
        if c >= ' ' && c <= '~' {
            let char_index = (c as usize) - 32; // 32 is the ASCII code for space
            
            if char_index < FONT_BITMAP.len() {
                let bitmap = FONT_BITMAP[char_index];
                
                // Draw the character
                for row in 0..7 {
                    for col in 0..5 {
                        // Check if this pixel is set in the bitmap
                        if (bitmap[row] & (0b10000000 >> col)) != 0 {
                            let pixel_x = cursor_x + col;
                            let pixel_y = y + row;
                            
                            // Check if the pixel is within bounds
                            if pixel_x < width {
                                let idx = pixel_y * width + pixel_x;
                                if idx < buffer.len() {
                                    buffer[idx] = color;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Move cursor to next position (6 pixels per character)
        cursor_x += 6;
        
        // Check if we've reached the end of the display area
        if cursor_x >= width - 6 {
            break;
        }
    }
}

/// Run the GUI application
pub fn run_gui(image_path: PathBuf, config: Config) -> Result<()> {
    println!("Starting GUI with image: {}", image_path.display());
    
    // Load the input image
    let input_image = load_image(&image_path)?;
    println!("Image loaded: {}x{}", input_image.image.width(), input_image.image.height());
    
    // Create resized image for GUI mode if configured
    let processed_image = if let Some(dimensions) = config.gui_resize_dimensions {
        println!("Resizing image for GUI mode to {}x{}", dimensions[0], dimensions[1]);
        resize_image(&input_image.image, dimensions)
    } else if let Some(dimensions) = config.resize_dimensions {
        println!("Resizing image to {}x{}", dimensions[0], dimensions[1]);
        resize_image(&input_image.image, dimensions)
    } else {
        input_image.image.clone()
    };
    
    // Create window
    let mut window = Window::new(
        "LeafComplexR Visualizer",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| LeafComplexError::Other(format!("Failed to create window: {}", e)))?;
    
    // Set window limits
    window.limit_update_rate(Some(Duration::from_millis(50))); // 20 FPS
    
    // Create GUI state
    let mut state = GuiState::new(processed_image, config);
    
    // Run initial analysis
    state.update_analysis()?;
    
    // Main loop
    println!("Entering main loop");
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Get mouse position
        if let Some((x, y)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
            state.mouse_x = x as usize;
            state.mouse_y = y as usize;
        }
        
        // Get mouse button state
        let mouse_down_now = window.get_mouse_down(minifb::MouseButton::Left);
        
        // Handle mouse click or drag
        if mouse_down_now {
            if !state.mouse_down {
                // Initial click
                if state.mouse_x < state.display_width {
                    // Click on image area - select contour point
                    if let Some(idx) = state.find_nearest_contour_point(state.mouse_x, state.mouse_y) {
                        if let Err(e) = state.select_point(idx) {
                            state.status_message = format!("Error selecting point: {}", e);
                        }
                    }
                } else if state.is_mouse_on_slider() {
                    // Click on slider - start dragging
                    state.slider_dragging = true;
                    if let Err(e) = state.handle_slider_movement() {
                        state.status_message = format!("Error updating kernel size: {}", e);
                    }
                }
            } else if state.slider_dragging {
                // Continue dragging slider
                if let Err(e) = state.handle_slider_movement() {
                    state.status_message = format!("Error updating kernel size: {}", e);
                }
            }
        } else {
            // Mouse up - stop dragging
            state.slider_dragging = false;
        }
        
        state.mouse_down = mouse_down_now;
        
        // Handle keyboard input with improved key repeat
        let now = Instant::now();
        
        // Handle single key presses without repeat
        if window.is_key_pressed(Key::T, minifb::KeyRepeat::No) {
            state.show_transparency = !state.show_transparency;
            state.status_message = format!("Transparency view: {}", 
                                         if state.show_transparency { "ON" } else { "OFF" });
        }
        
        if window.is_key_pressed(Key::C, minifb::KeyRepeat::No) {
            state.show_clr_regions = !state.show_clr_regions;
            state.status_message = format!("CLR regions view: {}", 
                                         if state.show_clr_regions { "ON" } else { "OFF" });
        }
    
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            state.show_right_spiral = !state.show_right_spiral;
            state.status_message = format!("Right spiral path: {}", 
                                       if state.show_right_spiral { "ON" } else { "OFF" });
        }
        
        // Handle H and L keys with improved repeat logic
        let handle_key_repeat = |key: Key, current_idx: Option<usize>, is_forward: bool| -> Result<()> {
            let contour_len = state.lec_contour.len();
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
            
            state.select_point(new_idx)
        };
        
        // Check if H is pressed (previous point)
        if window.is_key_down(Key::H) {
            let should_process = if let Some(last_key) = state.last_key_pressed {
                if last_key == Key::H {
                    // Check if enough time has passed for key repeat
                    if let Some(timer) = state.key_repeat_timer {
                        let elapsed = now.duration_since(timer);
                        // Initial delay, then faster repeat
                        let delay = if state.key_repeat_count == 0 {
                            Duration::from_millis(500) // Initial delay
                        } else {
                            Duration::from_millis(100) // Repeat delay
                        };
                        
                        elapsed > delay
                    } else {
                        true
                    }
                } else {
                    // New key pressed
                    true
                }
            } else {
                // No previous key
                true
            };
            
            if should_process {
                if let Err(e) = state.handle_key_repeat(Key::H, state.selected_point_idx, false) {
                    state.status_message = format!("Error selecting point: {}", e);
                }
                
                state.key_repeat_timer = Some(now);
                state.key_repeat_count += 1;
                state.last_key_pressed = Some(Key::H);
            }
        }
        // Check if L is pressed (next point)
        else if window.is_key_down(Key::L) {
            let should_process = if let Some(last_key) = state.last_key_pressed {
                if last_key == Key::L {
                    // Check if enough time has passed for key repeat
                    if let Some(timer) = state.key_repeat_timer {
                        let elapsed = now.duration_since(timer);
                        // Initial delay, then faster repeat
                        let delay = if state.key_repeat_count == 0 {
                            Duration::from_millis(500) // Initial delay
                        } else {
                            Duration::from_millis(100) // Repeat delay
                        };
                        
                        elapsed > delay
                    } else {
                        true
                    }
                } else {
                    // New key pressed
                    true
                }
            } else {
                // No previous key
                true
            };
            
            if should_process {
                if let Err(e) = state.handle_key_repeat(Key::L, state.selected_point_idx, true) {
                    state.status_message = format!("Error selecting point: {}", e);
                }
                
                state.key_repeat_timer = Some(now);
                state.key_repeat_count += 1;
                state.last_key_pressed = Some(Key::L);
            }
        }
        else {
            // No navigation keys pressed, reset state
            state.key_repeat_timer = None;
            state.key_repeat_count = 0;
            state.last_key_pressed = None;
        }
        
        // Update the buffer
        state.update_buffer();
        
        // Update the window
        window
            .update_with_buffer(&state.buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .map_err(|e| LeafComplexError::Other(format!("Failed to update window: {}", e)))?;
    }
    println!("GUI closed normally");
    Ok(())
}
