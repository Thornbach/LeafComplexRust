use image::RgbaImage;

use crate::errors::{LeafComplexError, Result};
use crate::path_algorithms::{
    calculate_golden_spiral_params, calculate_gyro_path_length, calculate_straight_path_length,
    calculate_clr_points, check_spiral_path_validity, check_straight_line_transparency,
    generate_golden_spiral_path, trace_straight_line, calculate_gyro_path_pink,
};

/// Result of marginal point analysis
#[derive(Debug, Clone)]
pub struct MarginalPointFeatures {
    pub point_index: usize,
    pub straight_path_length: f64,
    pub gyro_path_length: f64,
    pub gyro_path_perc: f64,
    pub clr_alpha: u32,
    pub clr_gamma: u32,
    pub gyro_path_pink: Option<u32>, // Only for LEC (Pink as Opaque)
}

/// Generate features for all marginal points
pub fn generate_features(
    reference_point: (u32, u32),
    marginal_points: &[(u32, u32)],
    image: &RgbaImage,
    marked_image: Option<&RgbaImage>,
    phi_exponent_factor: f64,
    marked_color: [u8; 3],
    spiral_steps: u32,
    is_lec: bool, // true for LEC (Pink as Opaque), false for LMC (Pink as Transparent)
) -> Result<Vec<MarginalPointFeatures>> {
    if marginal_points.is_empty() {
        return Err(LeafComplexError::NoValidPoints);
    }
    
    let mut features = Vec::with_capacity(marginal_points.len());
    
    // Select the appropriate image based on analysis type
    let analysis_image = if is_lec {
        // For LEC, use the marked image where pink regions are opaque
        marked_image.unwrap_or(image)
    } else {
        // For LMC, use the original image
        image
    };
    
    // Process each marginal point
    for (idx, &marginal_point) in marginal_points.iter().enumerate() {
        // Calculate straight path length
        let straight_path_length = calculate_straight_path_length(reference_point, marginal_point);
        
        // Trace straight line path
        let straight_line = trace_straight_line(reference_point, marginal_point);
        
        // Check if straight line crosses transparency
        let crosses_transparency = check_straight_line_transparency(&straight_line, analysis_image);
        
        // Initialize with default values
        let mut gyro_path_length = 0.0;
        let mut gyro_path_perc = 0.0;
        let mut clr_alpha = 0;
        let mut clr_gamma = 0;
        let mut gyro_path_pink = if is_lec { Some(0) } else { None };
        
        // If straight line crosses transparency, try to find a valid golden spiral path
        if crosses_transparency {
            // Calculate golden spiral parameters
            let (spiral_a_coeff, theta_contact) = 
                calculate_golden_spiral_params(straight_path_length, phi_exponent_factor);
            
            // Generate spiral path
            let spiral_path = generate_golden_spiral_path(
                reference_point,
                marginal_point,
                spiral_a_coeff,
                theta_contact,
                phi_exponent_factor,
                spiral_steps as usize,
            );
            
            // Check if spiral path is valid
            let spiral_valid = check_spiral_path_validity(&spiral_path, analysis_image);
            
            if spiral_valid {
                // Calculate golden spiral path length
                gyro_path_length = calculate_gyro_path_length(
                    spiral_a_coeff,
                    theta_contact,
                    phi_exponent_factor,
                );
                
                // Calculate percentage as ratio of gyro path length to straight path length
                gyro_path_perc = (gyro_path_length / straight_path_length) * 100.0;
                
                // Calculate CLR_Alpha and CLR_Gamma
                let (alpha, gamma) = calculate_clr_points(
                    reference_point,
                    marginal_point,
                    &spiral_path,
                    analysis_image,
                );
                
                clr_alpha = alpha;
                clr_gamma = gamma;
                
                // For LEC, count pink pixels along the spiral path
                if is_lec {
                    if let Some(marked) = marked_image {
                        gyro_path_pink = Some(calculate_gyro_path_pink(
                            &spiral_path,
                            marked,
                            marked_color,
                        ));
                    }
                }
            }
        }
        
        // Create features structure
        let point_features = MarginalPointFeatures {
            point_index: idx,
            straight_path_length,
            gyro_path_length,
            gyro_path_perc,
            clr_alpha,
            clr_gamma,
            gyro_path_pink,
        };
        
        features.push(point_features);
    }
    
    Ok(features)
}