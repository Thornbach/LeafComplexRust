use std::path::{Path, PathBuf};
use image::{RgbaImage, Rgba, imageops};
use eframe::egui;

use leaf_complex_rust_lib::{
    Config, morphology, shape_analysis, point_analysis, 
    feature_extraction, thornfiddle, load_image,
};


// Import from state.rs, not defining our own
use crate::state::{AnalysisResult, SummaryStats};

pub struct AnalysisEngine;

impl AnalysisEngine {
    pub fn new() -> Self {
        Self
    }
    
    // FIXED: Added the missing generate_thumbnail method
    pub fn generate_thumbnail(
        &self,
        image_path: &Path,
        ctx: &egui::Context,
    ) -> Option<egui::TextureHandle> {
        let input_image = load_image(image_path).ok()?;
        
        let thumbnail_size = 120;
        let (width, height) = input_image.image.dimensions();
        let aspect_ratio = width as f32 / height as f32;
        
        let (thumb_width, thumb_height) = if aspect_ratio > 1.0 {
            (thumbnail_size, (thumbnail_size as f32 / aspect_ratio) as u32)
        } else {
            ((thumbnail_size as f32 * aspect_ratio) as u32, thumbnail_size)
        };
        
        let thumbnail = imageops::resize(
            &input_image.image,
            thumb_width,
            thumb_height,
            imageops::FilterType::Lanczos3,
        );
        
        Some(load_texture_from_image(ctx, &thumbnail, format!("{}_thumb", input_image.filename)))
    }
    
    pub fn analyze_image(
        &self,
        image_path: &PathBuf,
        config: &Config,
        ctx: &egui::Context,
    ) -> Result<AnalysisResult, String> {
        println!("\n=== Starting Analysis ===");
        println!("Image: {:?}", image_path);
        
        let filename = image_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        
        let image = image::open(image_path)
            .map_err(|e| format!("Failed to load image: {}", e))?
            .to_rgba8();
        
        let processed_image = if let Some(dimensions) = config.resize_dimensions {
            image::imageops::resize(
                &image,
                dimensions[0],
                dimensions[1],
                image::imageops::FilterType::Lanczos3
            )
        } else {
            image
        };
        
        let adaptive_opening_kernel_size = calculate_adaptive_opening_kernel_size(
            &processed_image,
            config.adaptive_opening_max_density,
            config.adaptive_opening_max_percentage,
            config.adaptive_opening_min_percentage,
        );
        
        println!("Adaptive opening kernel size: {}", adaptive_opening_kernel_size);
        
        let opened_image = morphology::apply_opening(&processed_image, adaptive_opening_kernel_size)
            .map_err(|e| format!("Opening failed: {}", e))?;
        
        let mut marked_image = mark_opened_regions(
            &processed_image,
            &opened_image,
            config.marked_region_color_rgb,
        );
        
        // CRITICAL FIX: Clean thin artifacts (single-pixel lines that shouldn't be marked)
        marked_image = clean_thin_artifacts(&marked_image, config.marked_region_color_rgb);
        
        let mc_image = morphology::create_mc_with_com_component(
            &processed_image,
            &mut marked_image,
            config.marked_region_color_rgb,
        );
        
        println!("Created MC image");
        
        let ec_reference_point = point_analysis::get_reference_point(
            &processed_image,
            &marked_image,
            &config.reference_point_choice,
            config.marked_region_color_rgb,
        ).map_err(|e| format!("Failed to get EC reference point: {}", e))?;
        
        let mc_reference_point = point_analysis::get_mc_reference_point(
            &mc_image,
            &marked_image,
            &config.reference_point_choice,
            config.marked_region_color_rgb,
        ).map_err(|e| format!("Failed to get MC reference point: {}", e))?;
        
        println!("EC reference point: {:?}", ec_reference_point);
        println!("MC reference point: {:?}", mc_reference_point);
        
        // Trace ORIGINAL contours
        let ec_contour_original = morphology::trace_contour(&marked_image, true, config.marked_region_color_rgb);
        let mc_contour_original = morphology::trace_contour(&mc_image, false, config.marked_region_color_rgb);
        
        println!("Original EC contour points: {}", ec_contour_original.len());
        println!("Original MC contour points: {}", mc_contour_original.len());
        
        // Calculate metrics from ORIGINAL images with area parameter
        let ec_area = shape_analysis::calculate_area(&marked_image);
        let ec_outline_count = ec_contour_original.len() as u32;
        let ec_circularity = shape_analysis::calculate_circularity_from_contour(ec_area, &ec_contour_original);
        
        let mc_area = shape_analysis::calculate_area(&mc_image);
        let mc_outline_count = mc_contour_original.len() as u32;
        let mc_circularity = shape_analysis::calculate_circularity_from_contour(mc_area, &mc_contour_original);
        
        println!("EC metrics: Area={}, Outline={}, Circ={:.3}", ec_area, ec_outline_count, ec_circularity);
        println!("MC metrics: Area={}, Outline={}, Circ={:.3}", mc_area, mc_outline_count, mc_circularity);
        
        // Generate features from ORIGINAL contours
        let initial_ec_features = feature_extraction::generate_features(
            ec_reference_point,
            &ec_contour_original,
            &processed_image,
            Some(&marked_image),
            config.marked_region_color_rgb,
            true,
        ).map_err(|e| format!("EC feature extraction failed: {}", e))?;
        
        let initial_mc_features = feature_extraction::generate_features(
            mc_reference_point,
            &mc_contour_original,
            &mc_image,
            None,
            config.marked_region_color_rgb,
            false,
        ).map_err(|e| format!("MC feature extraction failed: {}", e))?;
        
        println!("Initial EC features: {}", initial_ec_features.len());
        println!("Initial MC features: {}", initial_mc_features.len());
        
        // Apply petiole filtering
        let (ec_features, _ec_petiole_info) = thornfiddle::filter_petiole_from_ec_features(
            &initial_ec_features,
            config.enable_petiole_filter_ec,
            config.petiole_remove_completely,
            1.0,
            config.enable_pink_threshold_filter,
            config.pink_threshold_value,
        );
        
        let (mc_features, _mc_petiole_info) = thornfiddle::filter_petiole_from_ec_features(
            &initial_mc_features,
            config.enable_petiole_filter_ec,
            config.petiole_remove_completely,
            1.0,
            false,
            0.0,
        );
        
        println!("After filtering - EC features: {}, MC features: {}", ec_features.len(), mc_features.len());
        
        // Create FILTERED contours matching FILTERED features
        let ec_contour_filtered: Vec<(u32, u32)> = ec_features.iter()
            .filter_map(|f| ec_contour_original.get(f.point_index))
            .copied()
            .collect();
        
        let mc_contour_filtered: Vec<(u32, u32)> = mc_features.iter()
            .filter_map(|f| mc_contour_original.get(f.point_index))
            .copied()
            .collect();
        
        println!("Filtered contours - EC: {}, MC: {}", ec_contour_filtered.len(), mc_contour_filtered.len());
        
        if ec_contour_filtered.len() != ec_features.len() {
            eprintln!("WARNING: EC contour/feature mismatch! Contour={}, Features={}", 
                     ec_contour_filtered.len(), ec_features.len());
        }
        if mc_contour_filtered.len() != mc_features.len() {
            eprintln!("WARNING: MC contour/feature mismatch! Contour={}, Features={}", 
                     mc_contour_filtered.len(), mc_features.len());
        }
        
        let (ec_length, ec_width, ec_shape_index) = shape_analysis::calculate_length_width_shape_index(
            &marked_image,
            config.marked_region_color_rgb,
        );
        
        println!("EC Shape: Length={:.1}, Width={:.1}, Index={:.3}, Circ={:.3}", 
                 ec_length, ec_width, ec_shape_index, ec_circularity);
        
        let (mc_length, mc_width, mc_shape_index, mc_shorter_dimension) = 
            shape_analysis::calculate_length_width_shape_index_with_shorter(
                &mc_image,
                config.marked_region_color_rgb,
            );
        
        println!("MC Shape: Length={:.1}, Width={:.1}, Index={:.3}, Circ={:.3}", 
                 mc_length, mc_width, mc_shape_index, mc_circularity);
        
        // Calculate dynamic opening for thornfiddle
        let dynamic_opening_percentage = shape_analysis::calculate_dynamic_opening_percentage(
            mc_shape_index,
            config.thornfiddle_max_opening_percentage,
            config.thornfiddle_min_opening_percentage,
        );
        
        let dynamic_kernel_size = ((dynamic_opening_percentage / 100.0) * mc_shorter_dimension)
            .round() as u32;
        let dynamic_kernel_size = dynamic_kernel_size.max(1);
        
        println!("Dynamic thornfiddle: MC Shape Index {:.3} -> {:.1}% -> {} px kernel", 
                 mc_shape_index, dynamic_opening_percentage, dynamic_kernel_size);
        
        // Create thornfiddle image
        let thornfiddle_image = morphology::create_thornfiddle_image(
            &mc_image,
            dynamic_kernel_size,
            config.thornfiddle_marked_color_rgb,
        ).map_err(|e| format!("Failed to create thornfiddle image: {}", e))?;
        
        let ec_circumference = thornfiddle::calculate_leaf_circumference(&ec_contour_original);
        
        // Calculate harmonic results
        let ec_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
            &ec_features,
            ec_circumference,
            &thornfiddle_image,
            ec_reference_point,
            &ec_contour_original,
            config.thornfiddle_marked_color_rgb,
            config.thornfiddle_pixel_threshold,
            config.harmonic_min_chain_length,
            config.harmonic_strength_multiplier,
            config.harmonic_max_harmonics,
        );
        
        let mc_circumference = thornfiddle::calculate_leaf_circumference(&mc_contour_original);
        
        let mc_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
            &mc_features,
            mc_circumference,
            &thornfiddle_image,
            mc_reference_point,
            &mc_contour_original,
            config.thornfiddle_marked_color_rgb,
            config.thornfiddle_pixel_threshold,
            config.harmonic_min_chain_length,
            config.harmonic_strength_multiplier,
            config.harmonic_max_harmonics,
        );
        
        println!("EC harmonic chains: {}", ec_harmonic_result.valid_chain_count);
        println!("MC harmonic chains: {}", mc_harmonic_result.valid_chain_count);
        
        let mut ec_features_final = ec_features.clone();
        for (i, feature) in ec_features_final.iter_mut().enumerate() {
            if let Some(&harmonic_value) = ec_harmonic_result.harmonic_values.get(i) {
                feature.thornfiddle_path_harmonic = harmonic_value;
            }
            feature.thornfiddle_path = thornfiddle::calculate_thornfiddle_path(feature);
        }
        
        let mut mc_features_final = mc_features.clone();
        for (i, feature) in mc_features_final.iter_mut().enumerate() {
            if let Some(&harmonic_value) = mc_harmonic_result.harmonic_values.get(i) {
                feature.thornfiddle_path_harmonic = harmonic_value;
            }
            feature.thornfiddle_path = thornfiddle::calculate_thornfiddle_path(feature);
        }
        
        // FIXED: Calculate MC spectral entropy with correct parameters
        // The function returns (entropy, smoothed_path), we only need entropy
        let mc_spectral_entropy = thornfiddle::calculate_spectral_entropy_from_harmonic_thornfiddle_path(
            &mc_features_final,
            mc_harmonic_result.valid_chain_count,
            config.thornfiddle_smoothing_strength,
            config.spectral_entropy_sigmoid_k,
            config.spectral_entropy_sigmoid_c,
        ).0;  // FIXED: Take only the first element (entropy value)
        
        let ec_approximate_entropy = thornfiddle::calculate_approximate_entropy_from_pink_path(
            &ec_features_final,
            config.approximate_entropy_m,
            config.approximate_entropy_r,
        );
        
        println!("EC Approximate Entropy: {:.6}", ec_approximate_entropy);
        println!("MC Spectral Entropy: {:.6}", mc_spectral_entropy);
        
        let ec_data: Vec<(f64, f64)> = ec_features_final.iter()
            .enumerate()
            .map(|(i, f)| (i as f64, f.diego_path_pink.unwrap_or(0) as f64))
            .collect();
        
        let mc_data: Vec<(f64, f64)> = mc_features_final.iter()
            .enumerate()
            .map(|(i, f)| (i as f64, f.thornfiddle_path_harmonic))
            .collect();
        
        println!("Graph data - EC: {} points (diego_path_pink), MC: {} points", ec_data.len(), mc_data.len());
        
        let ec_overlay = create_transparent_overlay(&marked_image, &[255, 0, 255]);
        let mc_overlay = create_transparent_overlay(&thornfiddle_image, &[255, 215, 0]);
        
        let original_texture = load_texture_from_image(ctx, &processed_image, format!("{}_original", filename));
        let ec_texture = load_texture_from_image(ctx, &ec_overlay, format!("{}_ec", filename));
        let mc_texture = load_texture_from_image(ctx, &mc_overlay, format!("{}_mc", filename));
        
        let summary = SummaryStats {
            ec_length,
            ec_width,
            ec_shape_index,
            ec_circularity,
            ec_spectral_entropy: ec_approximate_entropy,
            ec_area,
            ec_outline_count,
            mc_length,
            mc_width,
            mc_shape_index,
            mc_circularity,
            mc_spectral_entropy,
            mc_area,
            mc_outline_count,
        };
        
        println!("=== Analysis Complete ===");
        println!("Final verification:");
        println!("  EC: {} data points, {} contour points, {} features", 
                 ec_data.len(), ec_contour_filtered.len(), ec_features_final.len());
        println!("  MC: {} data points, {} contour points, {} features", 
                 mc_data.len(), mc_contour_filtered.len(), mc_features_final.len());
        println!();
        
        // FIXED: Return AnalysisResult matching state.rs structure
        Ok(AnalysisResult {
            ec_data,
            mc_data,
            summary,
            ec_image_texture: Some(ec_texture),
            mc_image_texture: Some(mc_texture),
            original_texture: Some(original_texture),
            ec_contour: ec_contour_filtered,
            mc_contour: mc_contour_filtered,
            ec_features: ec_features_final,
            mc_features: mc_features_final,
            ec_reference_point,
            mc_reference_point,
        })
    }
}

/// Remove thin artifacts from marked regions (single-pixel-wide lines)
/// Filters out any pink regions where pixels have <= 2 pink neighbors
fn clean_thin_artifacts(marked_image: &RgbaImage, marked_color: [u8; 3]) -> RgbaImage {
    let (width, height) = marked_image.dimensions();
    let mut cleaned = marked_image.clone();
    
    // Helper to check if pixel is pink
    let is_pink = |x: u32, y: u32| -> bool {
        if x >= width || y >= height {
            return false;
        }
        let pixel = marked_image.get_pixel(x, y);
        pixel[0] == marked_color[0] && 
        pixel[1] == marked_color[1] && 
        pixel[2] == marked_color[2] &&
        pixel[3] > 0
    };
    
    // Count pink neighbors in 8-connectivity
    let count_pink_neighbors = |x: u32, y: u32| -> usize {
        let mut count = 0;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 {
                    if is_pink(nx as u32, ny as u32) {
                        count += 1;
                    }
                }
            }
        }
        count
    };
    
    // First pass: mark pixels to remove
    let mut to_remove = vec![];
    for y in 0..height {
        for x in 0..width {
            if is_pink(x, y) {
                let neighbor_count = count_pink_neighbors(x, y);
                // Remove if 2 or fewer neighbors (thin line or isolated pixel)
                if neighbor_count <= 2 {
                    to_remove.push((x, y));
                }
            }
        }
    }
    
    // Second pass: remove marked pixels
    for (x, y) in to_remove.iter() {
        cleaned.put_pixel(*x, *y, Rgba([0, 0, 0, 0]));
    }
    
    println!("Cleaned {} thin artifact pixels from pink regions", to_remove.len());
    cleaned
}

fn calculate_adaptive_opening_kernel_size(
    image: &RgbaImage,
    max_density: f64,
    max_percentage: f64,
    min_percentage: f64,
) -> u32 {
    let (width, height) = image.dimensions();
    let total_pixels = (width * height) as f64;
    
    let mut non_transparent_count = 0;
    for pixel in image.pixels() {
        if pixel[3] > 0 {
            non_transparent_count += 1;
        }
    }
    
    let non_transparent_percentage = (non_transparent_count as f64 / total_pixels) * 100.0;
    
    let opening_percentage = if non_transparent_percentage >= max_density {
        max_percentage
    } else {
        let scale = non_transparent_percentage / max_density;
        min_percentage + (max_percentage - min_percentage) * scale
    };
    
    let smaller_dimension = width.min(height) as f64;
    let adaptive_kernel_size = ((opening_percentage / 100.0) * smaller_dimension).round() as u32;
    let adaptive_kernel_size = adaptive_kernel_size.max(1);
    
    println!("Adaptive opening: {:.1}% density -> {:.1}% opening -> {} px kernel", 
             non_transparent_percentage, opening_percentage, adaptive_kernel_size);
    
    adaptive_kernel_size
}

fn mark_opened_regions(
    original: &RgbaImage,
    opened: &RgbaImage,
    marked_color: [u8; 3],
) -> RgbaImage {
    let (width, height) = original.dimensions();
    let mut result = original.clone();
    
    for y in 0..height {
        for x in 0..width {
            let orig_pixel = original.get_pixel(x, y);
            let opened_pixel = opened.get_pixel(x, y);
            
            if orig_pixel[3] > 0 && opened_pixel[3] == 0 {
                result.put_pixel(x, y, Rgba([marked_color[0], marked_color[1], marked_color[2], 255]));
            }
        }
    }
    
    result
}

fn create_transparent_overlay(image: &RgbaImage, color_to_keep: &[u8; 3]) -> RgbaImage {
    let (width, height) = image.dimensions();
    let mut overlay = RgbaImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            
            if pixel[3] > 0 && pixel[0] == color_to_keep[0] && 
               pixel[1] == color_to_keep[1] && pixel[2] == color_to_keep[2] {
                overlay.put_pixel(x, y, *pixel);
            } else {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    
    overlay
}

fn load_texture_from_image(ctx: &egui::Context, image: &RgbaImage, name: String) -> egui::TextureHandle {
    let (width, height) = image.dimensions();
    let image_data: Vec<u8> = image.as_raw().clone();
    
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        &image_data,
    );
    
    ctx.load_texture(name, color_image, egui::TextureOptions::default())
}