// src/pipeline.rs - Updated with dynamic thornfiddle opening based on LMC Shape Index

use std::path::PathBuf;

use crate::config::Config;
use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::generate_features;
use crate::image_io::{InputImage, save_image};
use crate::image_utils::resize_image;
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour, create_lmc_with_com_component, create_thornfiddle_image};
use crate::output::{write_lec_csv, write_lmc_csv};
use crate::point_analysis::{get_reference_point, get_lmc_reference_point};
use crate::shape_analysis::{analyze_shape_comprehensive, calculate_length_width_shape_index, calculate_length_width_shape_index_with_shorter, calculate_dynamic_opening_percentage};
use crate::thornfiddle::{self, calculate_leaf_circumference};

/// Process a single image
pub fn process_image(
    input_image: InputImage,
    config: &Config,
    debug: bool,
) -> Result<()> {
    let InputImage { image, path, filename } = input_image;

    let subfolder = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("root");
    
    // Step 1: Resize if configured
    let processed_image = if let Some(dimensions) = config.resize_dimensions {
        resize_image(&image, dimensions)
    } else {
        image
    };
    
    // Step 2: Apply circular opening
    let opened_image = apply_opening(&processed_image, config.opening_kernel_size)?;
    
    // Step 2 (continued): Mark opened regions
    let mut marked_image = mark_opened_regions(
        &processed_image,
        &opened_image,
        config.marked_region_color_rgb,
    );

    let lmc_image = create_lmc_with_com_component(
        &processed_image,
        &mut marked_image, 
        config.marked_region_color_rgb
    );
    
    // STEP 2.5: Calculate LEC Shape Index (Length, Width, Shape Index)
    println!("Calculating LEC shape metrics...");
    let (lec_length, lec_width, lec_shape_index) = calculate_length_width_shape_index(
        &processed_image, 
        config.marked_region_color_rgb
    );
    
    if debug {
        println!("LEC Shape Analysis for {}:", filename);
        println!("  LEC Length: {:.1} pixels", lec_length);
        println!("  LEC Width: {:.1} pixels", lec_width);
        println!("  LEC Shape Index: {:.3}", lec_shape_index);
    }
    
    // STEP 2.6: Calculate LMC Shape Index (Length, Width, Shape Index, Shorter Dimension)
    println!("Calculating LMC shape metrics...");
    let (lmc_length, lmc_width, lmc_shape_index, lmc_shorter_dimension) = calculate_length_width_shape_index_with_shorter(
        &lmc_image, 
        config.marked_region_color_rgb
    );
    
    if debug {
        println!("LMC Shape Analysis for {}:", filename);
        println!("  LMC Length: {:.1} pixels", lmc_length);
        println!("  LMC Width: {:.1} pixels", lmc_width);
        println!("  LMC Shape Index: {:.3}", lmc_shape_index);
        println!("  LMC Shorter Dimension: {:.1} pixels", lmc_shorter_dimension);
    }
    
    // STEP 2.7: Calculate Dynamic Opening Percentage based on LMC Shape Index
    let dynamic_opening_percentage = calculate_dynamic_opening_percentage(
        lmc_shape_index,
        config.thornfiddle_max_opening_percentage,
        config.thornfiddle_min_opening_percentage,
    );
    
    // STEP 2.8: Calculate Dynamic Kernel Size from LMC SHORTER Dimension (CRITICAL FIX!)
    let dynamic_kernel_size = ((dynamic_opening_percentage / 100.0) * lmc_shorter_dimension).round() as u32;
    let dynamic_kernel_size = dynamic_kernel_size.max(1); // Ensure minimum of 1 pixel
    
    println!("Dynamic opening calculation: LMC Shape Index {:.3} -> {:.1}% -> {} pixel kernel (of {:.1} pixel SHORTER dimension)", 
             lmc_shape_index, dynamic_opening_percentage, dynamic_kernel_size, lmc_shorter_dimension);
    
    // Step 3: Create Thornfiddle Image with DYNAMIC golden lobe regions
    let thornfiddle_image = create_thornfiddle_image(
        &lmc_image,
        dynamic_kernel_size, // Use dynamic kernel size based on LMC SHORTER dimension
        config.thornfiddle_marked_color_rgb,
    )?;
    
    // Calculate comprehensive shape metrics including biological dimensions
    // For the original image: get area, circularity, length, width, outline count, and shape index
    let (area, lec_circularity, _orig_length, _orig_width, outline_count, _orig_shape_index) = 
        analyze_shape_comprehensive(&processed_image, config.marked_region_color_rgb);
    
    // For the LMC image: just get circularity (we already calculated length/width/shape_index above)
    let (_, lmc_circularity) = crate::shape_analysis::analyze_shape(&lmc_image, config.marked_region_color_rgb);
    
    if debug {
        println!("Shape analysis for {}:", filename);
        println!("  Area: {} pixels", area);
        println!("  Biological dimensions (LEC): {:.1} x {:.1} pixels", lec_length, lec_width);
        println!("  Biological dimensions (LMC): {:.1} x {:.1} pixels", lmc_length, lmc_width);
        println!("  Outline count: {} points", outline_count);
        println!("  LEC Circularity: {:.6}", lec_circularity);
        println!("  LMC Circularity: {:.6}", lmc_circularity);
        println!("  Dynamic Opening: {:.1}% -> {} pixel kernel (of {:.1} pixel LMC SHORTER dimension)", 
                 dynamic_opening_percentage, dynamic_kernel_size, lmc_shorter_dimension);
    }
    
    // Save debug images if requested
    if debug {
        let debug_dir = PathBuf::from(&config.output_base_dir).join("debug");
        std::fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
        
        save_image(&marked_image, debug_dir.join(format!("{}_marked.png", filename)))?;
        // Save Thornfiddle image showing golden lobe regions
        save_image(&thornfiddle_image, debug_dir.join(format!("{}_thornfiddle.png", filename)))?;
    }
    
    // Step 4: Calculate reference points - SEPARATE for LEC and LMC
    
    // LEC reference point (uses original processed image for COM if selected)
    let lec_reference_point = get_reference_point(
        &processed_image,
        &marked_image,
        &config.reference_point_choice,
        config.marked_region_color_rgb,
    )?;
    
    // LMC reference point (uses LMC image for COM if selected)
    let lmc_reference_point = get_lmc_reference_point(
        &lmc_image,
        &marked_image,
        &config.reference_point_choice,
        config.marked_region_color_rgb,
    )?;
    
    if debug {
        println!("LEC reference point for {}: {:?}", filename, lec_reference_point);
        println!("LMC reference point for {}: {:?}", filename, lmc_reference_point);
    }
    
    // Step 5: LEC Analysis (Pink regions are OPAQUE)
    let lec_contour = trace_contour(
        &marked_image,
        true, // is_pink_opaque = true for LEC
        config.marked_region_color_rgb,
    );
    
    // Generate initial LEC features
    let initial_lec_features = generate_features(
        lec_reference_point,
        &lec_contour,
        &processed_image,
        Some(&marked_image),
        config.golden_spiral_phi_exponent_factor,
        config.marked_region_color_rgb,
        config.golden_spiral_rotation_steps,
        true, // is_lec = true
    )?;
    
    // Apply petiole filtering to LEC features if enabled
    let (lec_features, petiole_info) = thornfiddle::filter_petiole_from_lec_features(
        &initial_lec_features,
        config.enable_petiole_filter_lec,
        config.petiole_remove_completely,
        1.0, // threshold for petiole detection
        config.enable_pink_threshold_filter,
        config.pink_threshold_value,
    );
    
    // UPDATED: Step 5.5: Calculate Golden Pixel Harmonic Thornfiddle Path for LEC features
    let lec_circumference = calculate_leaf_circumference(&lec_contour);
    let lec_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
        &lec_features,
        lec_circumference,
        &thornfiddle_image,
        lec_reference_point,
        &lec_contour,
        config.thornfiddle_marked_color_rgb,
        config.thornfiddle_pixel_threshold,
        config.harmonic_min_chain_length,
        config.harmonic_strength_multiplier,
    );
    
    // Update LEC features with harmonic values
    let mut lec_features_with_harmonic = lec_features;
    for (i, feature) in lec_features_with_harmonic.iter_mut().enumerate() {
        if let Some(&harmonic_value) = lec_harmonic_result.harmonic_values.get(i) {
            feature.thornfiddle_path_harmonic = harmonic_value;
        }
    }
    
    if debug {
        println!("LEC contour points: {}", lec_contour.len());
        if let Some(ref petiole_indices) = petiole_info {
            println!("Petiole detected: {} points", petiole_indices.len());
            if config.petiole_remove_completely {
                println!("Petiole handling: Complete removal (merging ends)");
            } else {
                println!("Petiole handling: Set to zero");
            }
        } else {
            println!("No petiole detected in LEC analysis");
        }
        
        if config.enable_pink_threshold_filter {
            println!("Pink threshold filter enabled: values <= {:.1} set to zero", config.pink_threshold_value);
        }
        
        println!("LEC harmonic chains: {} valid / {} total", 
                 lec_harmonic_result.valid_chain_count, lec_harmonic_result.total_chain_count);
    }
    
    // Step 6: LMC Analysis (Pink regions are TRANSPARENT) - Use LMC reference point
    let lmc_contour = trace_contour(
        &lmc_image,
        false, // is_pink_opaque = false for LMC
        config.marked_region_color_rgb,
    );
    
    let lmc_features = generate_features(
        lmc_reference_point, // Use LMC-specific reference point
        &lmc_contour,
        &lmc_image, // Use LMC image instead of processed image
        None, // No marked image needed for LMC
        config.golden_spiral_phi_exponent_factor,
        config.marked_region_color_rgb,
        config.golden_spiral_rotation_steps,
        false, // is_lec = false
    )?;
    
    // UPDATED: Step 6.5: Calculate Golden Pixel Harmonic Thornfiddle Path for LMC features  
    let lmc_circumference = calculate_leaf_circumference(&lmc_contour);
    let lmc_harmonic_result = thornfiddle::calculate_thornfiddle_path_harmonic(
        &lmc_features,
        lmc_circumference,
        &thornfiddle_image,
        lmc_reference_point,
        &lmc_contour,
        config.thornfiddle_marked_color_rgb,
        config.thornfiddle_pixel_threshold,
        config.harmonic_min_chain_length,
        config.harmonic_strength_multiplier,
    );
    
    // Update LMC features with harmonic values
    let mut lmc_features_with_harmonic = lmc_features;
    for (i, feature) in lmc_features_with_harmonic.iter_mut().enumerate() {
        if let Some(&harmonic_value) = lmc_harmonic_result.harmonic_values.get(i) {
            feature.thornfiddle_path_harmonic = harmonic_value;
        }
    }
    
    if debug {
        println!("LMC contour points: {}", lmc_contour.len());
        println!("LMC harmonic chains: {} valid / {} total", 
                 lmc_harmonic_result.valid_chain_count, lmc_harmonic_result.total_chain_count);
    }
    
    // Step 7: Write feature CSVs
    write_lec_csv(&lec_features_with_harmonic, &config.output_base_dir, &filename)?;
    
   // Step 8: Calculate spectral entropy from HARMONIC Thornfiddle Path (using LMC features)
   let (spectral_entropy, smoothed_thornfiddle_path) = thornfiddle::calculate_spectral_entropy_from_harmonic_thornfiddle_path(
    &lmc_features_with_harmonic,
    config.thornfiddle_smoothing_strength
);

    // Step 8b: Calculate spectral entropy from Pink Path (using LEC features, no smoothing)
    let spectral_entropy_pink = thornfiddle::calculate_spectral_entropy_from_pink_path(&lec_features_with_harmonic);

    // Step 8c: Calculate approximate entropy from Pink Path (using LEC features, respects petiole filtering)
    let approximate_entropy = thornfiddle::calculate_approximate_entropy_from_pink_path(
        &lec_features_with_harmonic,
        config.approximate_entropy_m,
        config.approximate_entropy_r,
    );

    // Step 8d: Calculate spectral entropy from LEC contour shape
    let spectral_entropy_contour = thornfiddle::calculate_spectral_entropy_from_contour(
        &lec_contour,
        config.thornfiddle_interpolation_points
    );

    // Step 8e: Calculate Edge Complexity from Pink Path values
    let pink_path_signal = thornfiddle::extract_pink_path_signal(&lec_features_with_harmonic);
    let edge_complexity = thornfiddle::calculate_edge_feature_density(
        &pink_path_signal,
        config.enable_petiole_filter_edge_complexity,
        config.petiole_remove_completely,
        config.lec_scaling_factor,
    ).unwrap_or_else(|e| {
        eprintln!("Warning: Edge complexity calculation failed for {}: {}", filename, e);
        0.0
    });

    // Write LMC CSV with smoothed Thornfiddle Path values
    write_lmc_csv(&lmc_features_with_harmonic, &config.output_base_dir, &filename, Some(&smoothed_thornfiddle_path))?;

    // UPDATED: Step 9: Create Thornfiddle summary with NEW shape index fields and dynamic kernel size
    // Use LMC harmonic chain count for the summary (as it's used for the main spectral entropy calculation)
    thornfiddle::create_thornfiddle_summary(
        &config.output_base_dir,
        &filename,
        subfolder,
        spectral_entropy,
        spectral_entropy_pink,
        spectral_entropy_contour,
        approximate_entropy,
        edge_complexity,
        lec_circularity,
        lmc_circularity,
        area,
        lec_length,         // LEC biological length
        lec_width,          // LEC biological width
        lec_shape_index,    // LEC Shape Index
        lmc_length,         // LMC biological length  
        lmc_width,          // LMC biological width
        lmc_shape_index,    // LMC Shape Index
        dynamic_opening_percentage, // Dynamic opening percentage used
        dynamic_kernel_size, // NEW: Dynamic kernel size used (in pixels)
        outline_count,      // outline point count
        lmc_harmonic_result.valid_chain_count, // harmonic chain count from LMC analysis
    )?;
    
    Ok(())
}