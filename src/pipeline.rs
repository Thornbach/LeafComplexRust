// src/pipeline.rs - Updated with separate COM/Circularity and path-based spectral entropy

use std::path::PathBuf;

use crate::config::Config;
use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::generate_features;
use crate::image_io::{InputImage, save_image};
use crate::image_utils::resize_image;
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour, create_lmc_with_com_component};
use crate::output::{write_lec_csv, write_lmc_csv};
use crate::point_analysis::{get_reference_point, get_lmc_reference_point};
use crate::shape_analysis::analyze_shape;
use crate::thornfiddle;

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
    
    // Calculate shape metrics for BOTH images
    let (area, lec_circularity) = analyze_shape(&processed_image, config.marked_region_color_rgb);
    let (_, lmc_circularity) = analyze_shape(&lmc_image, config.marked_region_color_rgb);
    
    // Save debug images if requested
    if debug {
        let debug_dir = PathBuf::from(&config.output_base_dir).join("debug");
        std::fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
        
        save_image(&processed_image, debug_dir.join(format!("{}_original.png", filename)))?;
        save_image(&opened_image, debug_dir.join(format!("{}_opened.png", filename)))?;
        save_image(&marked_image, debug_dir.join(format!("{}_marked.png", filename)))?;
        save_image(&lmc_image, debug_dir.join(format!("{}_lmc_image.png", filename)))?;
    }
    
    // Step 3: Calculate reference points - SEPARATE for LEC and LMC
    
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
    
    // Step 4: LEC Analysis (Pink regions are OPAQUE)
    let lec_contour = trace_contour(
        &marked_image,
        true, // is_pink_opaque = true for LEC
        config.marked_region_color_rgb,
    );
    
    let lec_features = generate_features(
        lec_reference_point,
        &lec_contour,
        &processed_image,
        Some(&marked_image),
        config.golden_spiral_phi_exponent_factor,
        config.marked_region_color_rgb,
        config.golden_spiral_rotation_steps,
        true, // is_lec = true
    )?;
    
    if debug {
        println!("LEC contour points: {}", lec_contour.len());
    }
    
    // Step 5: LMC Analysis (Pink regions are TRANSPARENT) - Use LMC reference point
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
    
    if debug {
        println!("LMC contour points: {}", lmc_contour.len());
    }
    
    // Step 6: Write feature CSVs
    write_lec_csv(&lec_features, &config.output_base_dir, &filename)?;
    
    // Step 7: Calculate spectral entropy from Thornfiddle Path (using LMC features)
    let (spectral_entropy, smoothed_thornfiddle_path) = thornfiddle::calculate_spectral_entropy_from_thornfiddle_path(
        &lmc_features,
        config.thornfiddle_smoothing_strength
    );
    
    // Step 7b: Calculate spectral entropy from Pink Path (using LEC features, no smoothing)
    let spectral_entropy_pink = thornfiddle::calculate_spectral_entropy_from_pink_path(&lec_features);
    
    // Step 7c: Calculate Edge Complexity from Pink Path values
    let pink_path_signal = thornfiddle::extract_pink_path_signal(&lec_features);
    let edge_complexity = thornfiddle::calculate_edge_feature_density(&pink_path_signal)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Edge complexity calculation failed for {}: {}", filename, e);
            0.0
        });
    
    // Write LMC CSV with smoothed Thornfiddle Path values
    write_lmc_csv(&lmc_features, &config.output_base_dir, &filename, Some(&smoothed_thornfiddle_path))?;

    // Step 8: Create Thornfiddle summary with all metrics
    thornfiddle::create_thornfiddle_summary(
        &config.output_base_dir,
        &filename,
        subfolder,
        spectral_entropy,
        spectral_entropy_pink,
        edge_complexity,
        lec_circularity,
        lmc_circularity,
        area,
    )?;
    
    // Debug output if requested
    if debug {
        thornfiddle::debug_thornfiddle_values(
            &lmc_features,
            &filename,
            &PathBuf::from(&config.output_base_dir),
            spectral_entropy,
            &smoothed_thornfiddle_path,
            config.thornfiddle_smoothing_strength,
        )?;
        
        println!("Analysis completed for {}:", filename);
        println!("  Spectral Entropy: {:.6}", spectral_entropy);
        println!("  Spectral Entropy Pink: {:.6}", spectral_entropy_pink);
        println!("  Edge Complexity: {:.6}", edge_complexity);
        println!("  LEC Circularity: {:.6}", lec_circularity);
        println!("  LMC Circularity: {:.6}", lmc_circularity);
        println!("  Area: {} pixels", area);
        println!("  LEC features: {}", lec_features.len());
        println!("  LMC features: {}", lmc_features.len());
        println!("  Reference points - LEC: {:?}, LMC: {:?}", lec_reference_point, lmc_reference_point);
    }
    
    Ok(())
}