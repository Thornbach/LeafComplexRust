// src/feature_extraction.rs - Simplified feature extraction for EC/MC analysis

use image::RgbaImage;

use crate::errors::{LeafComplexError, Result};
use crate::path_algorithms::{
    calculate_straight_path_length, calculate_diego_path, 
    calculate_diego_path_length, calculate_diego_path_pink, trace_straight_line,
    check_straight_line_transparency,
};

/// Represents features extracted from a single marginal (contour) point
#[derive(Debug, Clone)]
pub struct MarginalPointFeatures {
    /// Index of the point on the contour
    pub point_index: usize,
    
    /// Straight-line distance from reference point to marginal point (internal use only)
    pub straight_path_length: f64,
    
    /// Diego path length - geodesic distance staying within the leaf
    pub diego_path_length: f64,
    
    /// Number of pink pixels crossed by Diego path (EC analysis)
    pub diego_path_pink: Option<u32>,
    
    /// Thornfiddle path value - basic complexity measure
    pub thornfiddle_path: f64,
    
    /// Harmonic thornfiddle path value - enhanced with harmonic analysis
    pub thornfiddle_path_harmonic: f64,
}

/// Generate features for all marginal points on the contour
///
/// # Arguments
/// * `reference_point` - The reference point (COM or EP)
/// * `marginal_points` - All points on the leaf contour
/// * `image` - The processed image
/// * `marked_image` - Image with pink regions marked (for EC analysis)
/// * `marked_color` - RGB color used for marking
/// * `is_ec` - true for EC (pink as opaque), false for MC (pink as transparent)
///
/// # Returns
/// Vector of features for each marginal point
pub fn generate_features(
    reference_point: (u32, u32),
    marginal_points: &[(u32, u32)],
    image: &RgbaImage,
    marked_image: Option<&RgbaImage>,
    marked_color: [u8; 3],
    is_ec: bool,
) -> Result<Vec<MarginalPointFeatures>> {
    if marginal_points.is_empty() {
        return Err(LeafComplexError::NoValidPoints);
    }
    
    let mut features = Vec::with_capacity(marginal_points.len());
    
    // Select the appropriate image based on analysis type
    let analysis_image = if is_ec {
        // For EC, use the marked image where pink regions are opaque
        marked_image.unwrap_or(image)
    } else {
        // For MC, use the original image
        image
    };
    
    // Process each marginal point
    for (idx, &marginal_point) in marginal_points.iter().enumerate() {
        // Calculate straight path length (needed for internal calculations)
        let straight_path_length = calculate_straight_path_length(reference_point, marginal_point);
        
        // Trace straight line path
        let straight_line = trace_straight_line(reference_point, marginal_point);
        
        // Check if straight line crosses transparency
        let crosses_transparency = check_straight_line_transparency(&straight_line, analysis_image);
        
        // Calculate Diego Path - the shortest path that stays within the leaf
        let diego_path = if crosses_transparency {
            calculate_diego_path(reference_point, marginal_point, analysis_image)
        } else {
            straight_line.clone()
        };
        
        // Calculate Diego path length
        let diego_path_length = if crosses_transparency {
            calculate_diego_path_length(&diego_path)
        } else {
            straight_path_length // Use exact same value for consistency
        };
        
        // Calculate Diego path pink pixels (only for EC analysis)
        let diego_path_pink = if is_ec && !diego_path.is_empty() {
            if let Some(marked) = marked_image {
                Some(calculate_diego_path_pink(&diego_path, marked, marked_color))
            } else {
                None
            }
        } else {
            None
        };
        
        // Create features structure
        // Note: thornfiddle values will be calculated later
        let point_features = MarginalPointFeatures {
            point_index: idx,
            straight_path_length,
            diego_path_length,
            diego_path_pink,
            thornfiddle_path: 0.0, // Will be calculated later
            thornfiddle_path_harmonic: 0.0, // Will be calculated later
        };
        
        features.push(point_features);
    }
    
    Ok(features)
}
