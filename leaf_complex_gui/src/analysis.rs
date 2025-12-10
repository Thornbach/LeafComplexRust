// Analysis Engine - Bridges GUI and Backend
use std::path::Path;
use eframe::egui;
use image::RgbaImage;

use leaf_complex_rust_lib::{Config, load_image};
use crate::state::{AnalysisResult, SummaryStats};

pub struct AnalysisEngine;

impl AnalysisEngine {
    pub fn new() -> Self {
        Self
    }
    
    pub fn analyze_image(
        &self,
        image_path: &Path,
        config: &Config,
        ctx: &egui::Context,
    ) -> Result<AnalysisResult, String> {
        // Load image
        let input_image = load_image(image_path)
            .map_err(|e| format!("Failed to load image: {}", e))?;
        
        // Resize if needed
        let processed_image = if let Some(dimensions) = config.resize_dimensions {
            leaf_complex_rust_lib::image_utils::resize_image(&input_image.image, dimensions)
        } else {
            input_image.image.clone()
        };
        
        // Run full analysis pipeline (simplified version - you'll need to adapt)
        let result = self.run_analysis_pipeline(&processed_image, &input_image.filename, config, ctx)?;
        
        Ok(result)
    }
    
    fn run_analysis_pipeline(
        &self,
        image: &RgbaImage,
        filename: &str,
        config: &Config,
        ctx: &egui::Context,
    ) -> Result<AnalysisResult, String> {
        use leaf_complex_rust_lib::*;
        
        // Calculate adaptive opening
        let (width, height) = image.dimensions();
        let total_pixels = (width * height) as f64;
        
        let mut non_transparent_count = 0;
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y);
                if pixel[3] > 0 {
                    non_transparent_count += 1;
                }
            }
        }
        
        let non_transparent_percentage = (non_transparent_count as f64 / total_pixels) * 100.0;
        let opening_percentage = if non_transparent_percentage >= config.adaptive_opening_max_density {
            config.adaptive_opening_max_percentage
        } else {
            let scaling_factor = non_transparent_percentage / config.adaptive_opening_max_density;
            config.adaptive_opening_min_percentage + 
                scaling_factor * (config.adaptive_opening_max_percentage - config.adaptive_opening_min_percentage)
        };
        
        let image_dimension = std::cmp::min(width, height) as f64;
        let kernel_size = ((opening_percentage / 100.0) * image_dimension).round() as u32;
        let adaptive_kernel_size = kernel_size.max(1);
        
        // Apply opening
        let opened_image = morphology::apply_opening(image, adaptive_kernel_size)
            .map_err(|e| format!("Opening failed: {}", e))?;
        
        // Mark opened regions
        let mut marked_image = morphology::mark_opened_regions(
            image,
            &opened_image,
            config.marked_region_color_rgb,
        );
        
        // Create MC image
        let mc_image = morphology::create_mc_with_com_component(
            image,
            &mut marked_image,
            config.marked_region_color_rgb,
        );
        
        // Calculate shape metrics
        let (ec_length, ec_width, ec_shape_index) = 
            shape_analysis::calculate_length_width_shape_index(image, config.marked_region_color_rgb);
        
        let (mc_length, mc_width, mc_shape_index, mc_shorter_dimension) =
            shape_analysis::calculate_length_width_shape_index_with_shorter(&mc_image, config.marked_region_color_rgb);
        
        // Dynamic thornfiddle
        let dynamic_opening_percentage = shape_analysis::calculate_dynamic_opening_percentage(
            mc_shape_index,
            config.thornfiddle_max_opening_percentage,
            config.thornfiddle_min_opening_percentage,
        );
        
        let dynamic_kernel_size = ((dynamic_opening_percentage / 100.0) * mc_shorter_dimension)
            .round() as u32;
        let dynamic_kernel_size = dynamic_kernel_size.max(1);
        
        let thornfiddle_image = morphology::create_thornfiddle_image(
            &mc_image,
            dynamic_kernel_size,
            config.thornfiddle_marked_color_rgb,
        ).map_err(|e| format!("Thornfiddle creation failed: {}", e))?;
        
        // Get shape analysis
        let (area, _circularity, _, _, outline_count, _) =
            shape_analysis::analyze_shape_comprehensive(image, config.marked_region_color_rgb);
        
        // Get reference points
        let ec_reference_point = point_analysis::get_reference_point(
            image,
            &marked_image,
            &config.reference_point_choice,
            config.marked_region_color_rgb,
        ).map_err(|e| format!("EC reference point failed: {}", e))?;
        
        let mc_reference_point = point_analysis::get_mc_reference_point(
            &mc_image,
            &marked_image,
            &config.reference_point_choice,
            config.marked_region_color_rgb,
        ).map_err(|e| format!("MC reference point failed: {}", e))?;
        
        // Trace contours
        let ec_contour = morphology::trace_contour(
            &marked_image,
            true,
            config.marked_region_color_rgb,
        );
        
        let mc_contour = morphology::trace_contour(
            &mc_image,
            false,
            config.marked_region_color_rgb,
        );
        
        // Generate features
        let initial_ec_features = feature_extraction::generate_features(
            ec_reference_point,
            &ec_contour,
            image,
            Some(&marked_image),
            config.marked_region_color_rgb,
            true,
        ).map_err(|e| format!("EC feature extraction failed: {}", e))?;
        
        let (ec_features, _petiole_info) = thornfiddle::filter_petiole_from_ec_features(
            &initial_ec_features,
            config.enable_petiole_filter_ec,
            config.petiole_remove_completely,
            1.0,
            config.enable_pink_threshold_filter,
            config.pink_threshold_value,
        );
        
        let mc_features = feature_extraction::generate_features(
            mc_reference_point,
            &mc_contour,
            &mc_image,
            None,
            config.marked_region_color_rgb,
            false,
        ).map_err(|e| format!("MC feature extraction failed: {}", e))?;
        
        // Calculate harmonic enhancements
        let ec_circumference = thornfiddle::calculate_leaf_circumference(&ec_contour);
        let ec_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
            &ec_features,
            ec_circumference,
            &thornfiddle_image,
            ec_reference_point,
            &ec_contour,
            config.thornfiddle_marked_color_rgb,
            config.thornfiddle_pixel_threshold,
            config.harmonic_min_chain_length,
            config.harmonic_strength_multiplier,
            config.harmonic_max_harmonics,
        );
        
        let mc_circumference = thornfiddle::calculate_leaf_circumference(&mc_contour);
        let mc_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
            &mc_features,
            mc_circumference,
            &thornfiddle_image,
            mc_reference_point,
            &mc_contour,
            config.thornfiddle_marked_color_rgb,
            config.thornfiddle_pixel_threshold,
            config.harmonic_min_chain_length,
            config.harmonic_strength_multiplier,
            config.harmonic_max_harmonics,
        );
        
        // Update features with harmonic values
        let mut ec_features_final = ec_features;
        for (i, feature) in ec_features_final.iter_mut().enumerate() {
            if let Some(&harmonic_value) = ec_harmonic_result.harmonic_values.get(i) {
                feature.thornfiddle_path_harmonic = harmonic_value;
            }
            feature.thornfiddle_path = thornfiddle::calculate_thornfiddle_path(feature);
        }
        
        let mut mc_features_final = mc_features;
        for (i, feature) in mc_features_final.iter_mut().enumerate() {
            if let Some(&harmonic_value) = mc_harmonic_result.harmonic_values.get(i) {
                feature.thornfiddle_path_harmonic = harmonic_value;
            }
            feature.thornfiddle_path = thornfiddle::calculate_thornfiddle_path(feature);
        }
        
        // Calculate entropy metrics
        let mc_spectral_entropy = thornfiddle::calculate_spectral_entropy_from_harmonic_thornfiddle_path(
            &mc_features_final,
            mc_harmonic_result.valid_chain_count,
            config.thornfiddle_smoothing_strength,
            config.spectral_entropy_sigmoid_k,
            config.spectral_entropy_sigmoid_c,
        ).0;
        
        let ec_approximate_entropy = thornfiddle::calculate_approximate_entropy_from_pink_path(
            &ec_features_final,
            config.approximate_entropy_m,
            config.approximate_entropy_r,
        );
        
        // Extract data for plots
        let ec_data: Vec<(f64, f64)> = ec_features_final.iter()
            .map(|f| (f.point_index as f64, f.diego_path_pink.unwrap_or(0) as f64))
            .collect();
        
        let mc_data: Vec<(f64, f64)> = mc_features_final.iter()
            .map(|f| (f.point_index as f64, f.thornfiddle_path_harmonic))
            .collect();
        
        // Create textures for display
        let original_texture = load_texture_from_image(ctx, image, format!("{}_original", filename));
        let ec_texture = load_texture_from_image(ctx, &marked_image, format!("{}_ec", filename));
        let mc_texture = load_texture_from_image(ctx, &mc_image, format!("{}_mc", filename));
        
        // Create summary stats
        let summary = SummaryStats {
            ec_length,
            ec_width,
            ec_shape_index,
            ec_circularity: 0.0, // Would need to calculate
            ec_spectral_entropy: ec_approximate_entropy,
            mc_length,
            mc_width,
            mc_shape_index,
            mc_spectral_entropy,
            area,
            outline_count,
            harmonic_chain_count: mc_harmonic_result.valid_chain_count,
        };
        
        Ok(AnalysisResult {
            ec_data,
            mc_data,
            summary,
            ec_image_texture: Some(ec_texture),
            mc_image_texture: Some(mc_texture),
            original_texture: Some(original_texture),
        })
    }
}

/// Helper function to load texture from RgbaImage
fn load_texture_from_image(
    ctx: &egui::Context,
    image: &RgbaImage,
    name: String,
) -> egui::TextureHandle {
    let (width, height) = image.dimensions();
    let pixels: Vec<egui::Color32> = image
        .pixels()
        .map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    
    let color_image = egui::ColorImage {
        size: [width as usize, height as usize],
        pixels,
    };
    
    ctx.load_texture(name, color_image, egui::TextureOptions::default())
}
