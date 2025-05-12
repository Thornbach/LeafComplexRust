use image::RgbaImage;

use crate::errors::{LeafComplexError, Result};
use crate::path_algorithms::{
    calculate_golden_spiral_params, calculate_gyro_path_length, calculate_straight_path_length,
    calculate_clr_points, check_spiral_path_validity, check_straight_line_transparency,
    generate_golden_spiral_path, trace_straight_line, calculate_gyro_path_pink,
    generate_left_right_spirals, calculate_diego_path, calculate_diego_path_length,
    calculate_diego_path_pink,
};


#[derive(Debug, Clone)]
pub struct MarginalPointFeatures {
    pub point_index: usize,
    pub straight_path_length: f64,
    pub gyro_path_length: f64,
    pub gyro_path_perc: f64,
    pub clr_alpha: u32,
    pub clr_gamma: u32,
    // Added fields for left and right spiral paths
    pub left_clr_alpha: u32,
    pub left_clr_gamma: u32,
    pub right_clr_alpha: u32,
    pub right_clr_gamma: u32,
    pub gyro_path_pink: Option<u32>, // Average pink pixels
    pub left_gyro_path_pink: Option<u32>, // Only for LEC (Pink as Opaque) - left path
    pub right_gyro_path_pink: Option<u32>, // Only for LEC (Pink as Opaque) - right path
    
    // New DiegoPath fields
    pub diego_path_length: f64,
    pub diego_path_perc: f64,
    pub diego_path_pink: Option<u32>, // Pink pixels along the diego path
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
        
        // New fields for left and right spiral paths
        let mut left_clr_alpha = 0;
        let mut left_clr_gamma = 0;
        let mut right_clr_alpha = 0;
        let mut right_clr_gamma = 0;
        
        let mut gyro_path_pink = if is_lec { Some(0) } else { None };
        let mut left_gyro_path_pink = if is_lec { Some(0) } else { None };
        let mut right_gyro_path_pink = if is_lec { Some(0) } else { None };
        
        // Calculate DiegoPath - the shortest path that stays within the leaf
        let diego_path = calculate_diego_path(reference_point, marginal_point, analysis_image);
        let diego_path_length = calculate_diego_path_length(&diego_path);
        let diego_path_perc = if straight_path_length > 0.0 {
            (diego_path_length / straight_path_length) * 100.0
        } else {
            0.0
        };
        
        // Calculate DiegoPath pink pixels
        let diego_path_pink = if is_lec && !diego_path.is_empty() {
            if let Some(marked) = marked_image {
                Some(calculate_diego_path_pink(&diego_path, marked, marked_color))
            } else {
                None
            }
        } else {
            None
        };
        
        // If straight line crosses transparency, try to find a valid golden spiral path
        if crosses_transparency {
            // Calculate golden spiral parameters
            let (spiral_a_coeff, theta_contact) = 
                calculate_golden_spiral_params(straight_path_length, phi_exponent_factor);
            
            // Generate both left and right spiral paths
            let (left_spiral_path, right_spiral_path) = generate_left_right_spirals(
                reference_point,
                marginal_point,
                spiral_a_coeff,
                theta_contact,
                phi_exponent_factor,
                spiral_steps as usize,
            );
            
            // Check if left spiral path is valid
            let left_spiral_valid = check_spiral_path_validity(&left_spiral_path, analysis_image);
            
            // Check if right spiral path is valid
            let right_spiral_valid = check_spiral_path_validity(&right_spiral_path, analysis_image);
            
            // Process the spiral paths if at least one is valid
            if left_spiral_valid || right_spiral_valid {
                // Calculate golden spiral path length
                gyro_path_length = calculate_gyro_path_length(
                    spiral_a_coeff,
                    theta_contact,
                    phi_exponent_factor,
                );
                
                // Calculate percentage as ratio of gyro path length to straight path length
                gyro_path_perc = (gyro_path_length / straight_path_length) * 100.0;
                
                // Calculate CLR values for left spiral path
                if left_spiral_valid {
                    let (alpha, gamma) = calculate_clr_points(
                        reference_point,
                        marginal_point,
                        &left_spiral_path,
                        analysis_image,
                    );
                    
                    left_clr_alpha = alpha;
                    left_clr_gamma = gamma;
                }
                
                // Calculate CLR values for right spiral path
                if right_spiral_valid {
                    let (alpha, gamma) = calculate_clr_points(
                        reference_point,
                        marginal_point,
                        &right_spiral_path,
                        analysis_image,
                    );
                    
                    right_clr_alpha = alpha;
                    right_clr_gamma = gamma;
                }
                
                // Calculate averaged CLR values
                if left_spiral_valid && right_spiral_valid {
                    // Average both left and right values
                    clr_alpha = ((left_clr_alpha as f64 + right_clr_alpha as f64) / 2.0).floor() as u32;
                    clr_gamma = ((left_clr_gamma as f64 + right_clr_gamma as f64) / 2.0).floor() as u32;
                } else if left_spiral_valid {
                    // Use only left values
                    clr_alpha = left_clr_alpha;
                    clr_gamma = left_clr_gamma;
                } else if right_spiral_valid {
                    // Use only right values
                    clr_alpha = right_clr_alpha;
                    clr_gamma = right_clr_gamma;
                }
                
                // For LEC, count pink pixels along the spiral path
                if is_lec {
                    if let Some(marked) = marked_image {
                        // Calculate pink pixels for left spiral
                        if left_spiral_valid {
                            left_gyro_path_pink = Some(calculate_gyro_path_pink(
                                &left_spiral_path,
                                marked,
                                marked_color,
                            ));
                        }
                        
                        // Calculate pink pixels for right spiral
                        if right_spiral_valid {
                            right_gyro_path_pink = Some(calculate_gyro_path_pink(
                                &right_spiral_path,
                                marked,
                                marked_color,
                            ));
                        }
                        
                        // Calculate average pink pixel count
                        if left_spiral_valid && right_spiral_valid {
                            // Average both left and right values
                            let left_pink = left_gyro_path_pink.unwrap_or(0);
                            let right_pink = right_gyro_path_pink.unwrap_or(0);
                            gyro_path_pink = Some(((left_pink as f64 + right_pink as f64) / 2.0).floor() as u32);
                        } else if left_spiral_valid {
                            // Use only left value
                            gyro_path_pink = left_gyro_path_pink;
                        } else if right_spiral_valid {
                            // Use only right value
                            gyro_path_pink = right_gyro_path_pink;
                        }
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
            left_clr_alpha,
            left_clr_gamma,
            right_clr_alpha,
            right_clr_gamma,
            gyro_path_pink,
            left_gyro_path_pink,
            right_gyro_path_pink,
            diego_path_length,
            diego_path_perc,
            diego_path_pink,
        };
        
        features.push(point_features);
    }
    
    Ok(features)
}