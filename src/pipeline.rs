// src/pipeline.rs - Updated to use simplified spectral entropy

use std::path::PathBuf;

use crate::config::Config;
use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::generate_features;
use crate::image_io::{InputImage, save_image};
use crate::image_utils::resize_image;
use crate::morphology::{apply_opening, mark_opened_regions, trace_contour, create_lmc_with_com_component};
use crate::output::{write_lec_csv, write_lmc_csv};
use crate::point_analysis::get_reference_point;
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
    
    // Get shape metrics
    let (area, circularity) = analyze_shape(&processed_image, config.marked_region_color_rgb);
    
    // Save debug images if requested
    if debug {
        let debug_dir = PathBuf::from(&config.output_base_dir).join("debug");
        std::fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
        
        save_image(&processed_image, debug_dir.join(format!("{}_original.png", filename)))?;
        save_image(&opened_image, debug_dir.join(format!("{}_opened.png", filename)))?;
        save_image(&marked_image, debug_dir.join(format!("{}_marked.png", filename)))?;
        save_image(&lmc_image, debug_dir.join(format!("{}_lmc_image.png", filename)))?;
    }
    
    // Step 3: Calculate reference point
    let reference_point = get_reference_point(
        &processed_image,
        &marked_image,
        &config.reference_point_choice,
        config.marked_region_color_rgb,
    )?;
    
    if debug {
        println!("Reference point for {}: {:?}", filename, reference_point);
    }
    
    // Step 4: LEC Analysis (Pink regions are OPAQUE)
    let lec_contour = trace_contour(
        &marked_image,
        true, // is_pink_opaque = true for LEC
        config.marked_region_color_rgb,
    );
    
    let lec_features = generate_features(
        reference_point,
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
    
    // Step 5: LMC Analysis (Pink regions are TRANSPARENT)
    let lmc_contour = trace_contour(
        &lmc_image,
        false, // is_pink_opaque = false for LMC
        config.marked_region_color_rgb,
    );
    
    let lmc_features = generate_features(
        reference_point,
        &lmc_contour,
        &processed_image,
        Some(&marked_image),
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
    write_lmc_csv(&lmc_features, &config.output_base_dir, &filename)?;

    // Step 7: Calculate spectral entropy (simplified)
    let spectral_entropy = thornfiddle::calculate_spectral_entropy(
        &lmc_contour,
        config.thornfiddle_interpolation_points
    );
    
    // Step 8: Create Thornfiddle summary
    thornfiddle::create_thornfiddle_summary(
        &config.output_base_dir,
        &filename,
        subfolder,
        spectral_entropy,
        circularity,
        area
    )?;
    
    // Debug output if requested
    if debug {
        thornfiddle::debug_thornfiddle_values(
            &lmc_features,
            &filename,
            &PathBuf::from(&config.output_base_dir),
            spectral_entropy
        )?;
        
        println!("Analysis completed for {}:", filename);
        println!("  Spectral Entropy: {:.6}", spectral_entropy);
        println!("  Circularity: {:.6}", circularity);
        println!("  Area: {} pixels", area);
        println!("  LEC features: {}", lec_features.len());
        println!("  LMC features: {}", lmc_features.len());
    }
    
    Ok(())
}