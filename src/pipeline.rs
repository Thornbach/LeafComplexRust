// src/pipeline.rs - Updated with Golden Pixel Thornfiddle implementation

use std::path::PathBuf;

use crate::config::Config;
use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::generate_features;
use crate::image_io::{InputImage, save_image};
use crate::image_utils::resize_image;
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour, create_lmc_with_com_component, create_thornfiddle_image};
use crate::output::{write_lec_csv, write_lmc_csv};
use crate::point_analysis::{get_reference_point, get_lmc_reference_point};
use crate::shape_analysis::analyze_shape_comprehensive;
use crate::thornfiddle::{self, calculate_thornfiddle_path_harmonic, calculate_leaf_circumference};

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
    
    // NEW: Step 3: Create Thornfiddle Image with golden lobe regions
    let thornfiddle_image = create_thornfiddle_image(
        &lmc_image,
        config.thornfiddle_opening_size_percentage,
        config.thornfiddle_marked_color_rgb,
    )?;
    
    // Calculate comprehensive shape metrics including biological dimensions
    // For the original image: get area, circularity, length, width, and outline count
    let (area, lec_circularity, length, width, outline_count) = 
        analyze_shape_comprehensive(&processed_image, config.marked_region_color_rgb);
    
    // For the LMC image: just get circularity (we already have the other measurements from original)
    let (_, lmc_circularity) = crate::shape_analysis::analyze_shape(&lmc_image, config.marked_region_color_rgb);
    
    if debug {
        println!("Shape analysis for {}:", filename);
        println!("  Area: {} pixels", area);
        println!("  Biological dimensions: {:.1} x {:.1} pixels", length, width);
        println!("  Outline count: {} points", outline_count);
        println!("  LEC Circularity: {:.6}", lec_circularity);
        println!("  LMC Circularity: {:.6}", lmc_circularity);
    }
    
    // Save debug images if requested
    if debug {
        let debug_dir = PathBuf::from(&config.output_base_dir).join("debug");
        std::fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
        
        //save_image(&processed_image, debug_dir.join(format!("{}_original.png", filename)))?;
        //save_image(&opened_image, debug_dir.join(format!("{}_opened.png", filename)))?;
        save_image(&marked_image, debug_dir.join(format!("{}_marked.png", filename)))?;
        //save_image(&lmc_image, debug_dir.join(format!("{}_lmc_image.png", filename)))?;
        // NEW: Save Thornfiddle image showing golden lobe regions
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
    
    // NEW: Step 5.5: Calculate Golden Pixel Harmonic Thornfiddle Path for LEC features
    let lec_circumference = calculate_leaf_circumference(&lec_contour);
    let lec_harmonic_values = calculate_thornfiddle_path_harmonic(
        &lec_features,
        lec_circumference,
        &thornfiddle_image,
        lec_reference_point,
        &lec_contour,
        config.thornfiddle_marked_color_rgb,
        config.thornfiddle_pixel_threshold,
    );
    
    // Update LEC features with harmonic values
    let mut lec_features_with_harmonic = lec_features;
    for (i, feature) in lec_features_with_harmonic.iter_mut().enumerate() {
        if let Some(&harmonic_value) = lec_harmonic_values.get(i) {
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
    
    // NEW: Step 6.5: Calculate Golden Pixel Harmonic Thornfiddle Path for LMC features  
    let lmc_circumference = calculate_leaf_circumference(&lmc_contour);
    let lmc_harmonic_values = calculate_thornfiddle_path_harmonic(
        &lmc_features,
        lmc_circumference,
        &thornfiddle_image,
        lmc_reference_point,
        &lmc_contour,
        config.thornfiddle_marked_color_rgb,
        config.thornfiddle_pixel_threshold,
    );
    
    // Update LMC features with harmonic values
    let mut lmc_features_with_harmonic = lmc_features;
    for (i, feature) in lmc_features_with_harmonic.iter_mut().enumerate() {
        if let Some(&harmonic_value) = lmc_harmonic_values.get(i) {
            feature.thornfiddle_path_harmonic = harmonic_value;
        }
    }
    
    if debug {
        println!("LMC contour points: {}", lmc_contour.len());
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

    // Step 9: Create Thornfiddle summary with all metrics INCLUDING biological dimensions and outline measurements
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
        length,         // NEW: biological length (longest distance between contour points)
        width,          // NEW: biological width (perpendicular to length axis)
        outline_count,  // outline point count
    )?;

    // Debug output if requested
    if debug {
        thornfiddle::debug_thornfiddle_values(
            &lmc_features_with_harmonic,
            &filename,
            &PathBuf::from(&config.output_base_dir),
            spectral_entropy,
            &smoothed_thornfiddle_path,
            config.thornfiddle_smoothing_strength,
        )?;
        
        println!("Golden Pixel Thornfiddle analysis for {}:", filename);
        println!("  LEC circumference: {:.1} pixels", lec_circumference);
        println!("  LMC circumference: {:.1} pixels", lmc_circumference);
        println!("  Thornfiddle opening size: {:.1}% of image width", config.thornfiddle_opening_size_percentage);
        println!("  Golden pixel threshold: {} pixels", config.thornfiddle_pixel_threshold);
        
        // Show harmonic enhancement statistics
        if !lec_harmonic_values.is_empty() {
            let lec_min = lec_harmonic_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let lec_max = lec_harmonic_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let lec_mean = lec_harmonic_values.iter().sum::<f64>() / lec_harmonic_values.len() as f64;
            println!("  LEC harmonic - min: {:.6}, max: {:.6}, mean: {:.6}", lec_min, lec_max, lec_mean);
        }
        
        if !lmc_harmonic_values.is_empty() {
            let lmc_min = lmc_harmonic_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let lmc_max = lmc_harmonic_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let lmc_mean = lmc_harmonic_values.iter().sum::<f64>() / lmc_harmonic_values.len() as f64;
            println!("  LMC harmonic - min: {:.6}, max: {:.6}, mean: {:.6}", lmc_min, lmc_max, lmc_mean);
        }
        
        println!("Analysis completed for {}:", filename);
        println!("  Spectral Entropy (Golden Harmonic): {:.6}", spectral_entropy);
        println!("  Spectral Entropy Pink: {:.6}", spectral_entropy_pink);
        println!("  Spectral Entropy Contour: {:.6}", spectral_entropy_contour);
        println!("  Approximate Entropy: {:.6}", approximate_entropy);
        println!("  Edge Complexity: {:.6}", edge_complexity);
        println!("  LEC Circularity: {:.6}", lec_circularity);
        println!("  LMC Circularity: {:.6}", lmc_circularity);
        println!("  Area: {} pixels", area);
        println!("  Biological Dimensions: {:.1} x {:.1} pixels", length, width);
        println!("  Outline Count: {} points", outline_count);
        println!("  LEC features: {}", lec_features_with_harmonic.len());
        println!("  LMC features: {}", lmc_features_with_harmonic.len());
        println!("  Reference points - LEC: {:?}, LMC: {:?}", lec_reference_point, lmc_reference_point);
    }
    
    Ok(())
}