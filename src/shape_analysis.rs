// src/shape_analysis.rs - Fixed version without compilation errors

use image::RgbaImage;
use crate::image_utils::is_non_transparent;
use crate::morphology::{trace_contour, smooth_contour};
use std::f64::consts::PI;

/// Calculate the area of non-transparent pixels in the image
pub fn calculate_area(image: &RgbaImage) -> u32 {
    let (width, height) = image.dimensions();
    let mut non_transparent_count = 0;
    
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            if is_non_transparent(pixel) {
                non_transparent_count += 1;
            }
        }
    }
    
    non_transparent_count
}

/// Calculate the bounding box dimensions of non-transparent pixels
/// Returns (width, height) of the bounding box
pub fn calculate_bounding_box_dimensions(image: &RgbaImage) -> (u32, u32) {
    let (img_width, img_height) = image.dimensions();
    
    let mut min_x = img_width;
    let mut max_x = 0;
    let mut min_y = img_height;
    let mut max_y = 0;
    let mut found_pixels = false;
    
    // Find the bounding box of all non-transparent pixels
    for y in 0..img_height {
        for x in 0..img_width {
            let pixel = image.get_pixel(x, y);
            if is_non_transparent(pixel) {
                found_pixels = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    
    if !found_pixels {
        return (0, 0);
    }
    
    // Calculate dimensions
    let width = if max_x >= min_x { max_x - min_x + 1 } else { 0 };
    let height = if max_y >= min_y { max_y - min_y + 1 } else { 0 };
    
    (width, height)
}

/// Calculate the outline count from a pre-computed contour
pub fn calculate_outline_count_from_contour(contour: &[(u32, u32)]) -> u32 {
    contour.len() as u32
}

/// Calculate the outline count (number of contour points) from the original image
pub fn calculate_outline_count(image: &RgbaImage, marked_color: [u8; 3]) -> u32 {
    // Trace the contour of the original image (treating pink as opaque)
    let contour = trace_contour(image, true, marked_color);
    contour.len() as u32
}

/// Calculate biological length and width from contour points
/// Length = longest straight-line distance between any two contour points
/// Width = maximum perpendicular distance to the length axis
pub fn calculate_biological_dimensions(contour: &[(u32, u32)]) -> (f64, f64) {
    if contour.len() < 2 {
        return (0.0, 0.0);
    }
    
    // Find the two points with maximum distance (length)
    let mut max_length = 0.0;
    let mut length_p1 = (0.0, 0.0);
    let mut length_p2 = (0.0, 0.0);
    
    for i in 0..contour.len() {
        for j in (i + 1)..contour.len() {
            let p1 = (contour[i].0 as f64, contour[i].1 as f64);
            let p2 = (contour[j].0 as f64, contour[j].1 as f64);
            
            let distance = ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
            
            if distance > max_length {
                max_length = distance;
                length_p1 = p1;
                length_p2 = p2;
            }
        }
    }
    
    // Calculate the direction vector of the length axis
    let length_vec = (length_p2.0 - length_p1.0, length_p2.1 - length_p1.1);
    let length_vec_normalized = {
        let len = (length_vec.0.powi(2) + length_vec.1.powi(2)).sqrt();
        if len > 0.0 {
            (length_vec.0 / len, length_vec.1 / len)
        } else {
            (1.0, 0.0)
        }
    };
    
    // Find maximum width (perpendicular distance to length axis)
    let mut min_width: f64 = 0.0;
    let mut max_width: f64 = 0.0;
    
    for point in contour {
        let p = (point.0 as f64, point.1 as f64);
        
        // Calculate perpendicular distance from point to length axis
        let to_point = (p.0 - length_p1.0, p.1 - length_p1.1);
        
        // Project onto the perpendicular direction
        let perp_vec = (-length_vec_normalized.1, length_vec_normalized.0);
        let perp_distance = to_point.0 * perp_vec.0 + to_point.1 * perp_vec.1;
        
        min_width = min_width.min(perp_distance);
        max_width = max_width.max(perp_distance);
    }
    
    // Width is the total span between min and max
    let width = max_width - min_width;
    
    (max_length, width)
}

/// Fast biological dimensions (optimized version for better performance)
/// Uses sampling for very large contours to reduce O(n²) complexity
pub fn calculate_biological_dimensions_fast(contour: &[(u32, u32)]) -> (f64, f64) {
    if contour.len() < 2 {
        return (0.0, 0.0);
    }
    
    // For performance, sample fewer points if contour is very large
    // This reduces complexity from O(n²) to O(s²) where s is sample size
    let sample_step = if contour.len() > 500 { 
        std::cmp::max(1, contour.len() / 250) 
    } else { 
        1 
    };
    
    let mut max_length = 0.0;
    let mut length_p1 = (0.0, 0.0);
    let mut length_p2 = (0.0, 0.0);
    
    // Sample points for length calculation
    for i in (0..contour.len()).step_by(sample_step) {
        for j in ((i + contour.len()/4)..contour.len()).step_by(sample_step) {
            let p1 = (contour[i].0 as f64, contour[i].1 as f64);
            let p2 = (contour[j].0 as f64, contour[j].1 as f64);
            
            let distance = ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
            
            if distance > max_length {
                max_length = distance;
                length_p1 = p1;
                length_p2 = p2;
            }
        }
    }
    
    // Calculate perpendicular width using all points for accuracy
    let length_vec = (length_p2.0 - length_p1.0, length_p2.1 - length_p1.1);
    let length_vec_normalized = {
        let len = (length_vec.0.powi(2) + length_vec.1.powi(2)).sqrt();
        if len > 0.0 {
            (length_vec.0 / len, length_vec.1 / len)
        } else {
            (1.0, 0.0)
        }
    };
    
    let mut min_width: f64 = 0.0;
    let mut max_width: f64 = 0.0;
    
    // Use a smaller sample step for width calculation to maintain accuracy
    let width_sample_step = std::cmp::max(1, sample_step / 2);
    
    for point in contour.iter().step_by(width_sample_step) {
        let p = (point.0 as f64, point.1 as f64);
        let to_point = (p.0 - length_p1.0, p.1 - length_p1.1);
        let perp_vec = (-length_vec_normalized.1, length_vec_normalized.0);
        let perp_distance = to_point.0 * perp_vec.0 + to_point.1 * perp_vec.1;
        
        min_width = min_width.min(perp_distance);
        max_width = max_width.max(perp_distance);
    }
    
    let width = max_width - min_width;
    
    (max_length, width)
}

/// Calculate the perimeter of the leaf using contour points
pub fn calculate_perimeter(contour_points: &[(u32, u32)]) -> f64 {
    if contour_points.len() < 2 {
        return 0.0;
    }
    
    let mut perimeter = 0.0;
    let n = contour_points.len();
    
    for i in 0..n {
        let (x1, y1) = contour_points[i];
        let (x2, y2) = contour_points[(i + 1) % n]; // Wrap around to first point
        
        let dx = x2 as f64 - x1 as f64;
        let dy = y2 as f64 - y1 as f64;
        perimeter += (dx * dx + dy * dy).sqrt();
    }
    
    perimeter
}

/// Apply correction factor to adjust for digitization artifacts in perimeter calculation
pub fn correct_perimeter(perimeter: f64, circularity_estimate: f64) -> f64 {
    // Apply correction based on how circle-like the shape appears to be
    if circularity_estimate > 0.75 {
        // For circle-like shapes, apply stronger correction
        // The correction factor is empirically determined
        perimeter * 0.945
    } else if circularity_estimate > 0.5 {
        // For somewhat circular shapes, apply moderate correction
        perimeter * 0.975
    } else {
        // For non-circular shapes, apply minimal correction
        perimeter * 0.99
    }
}

/// Calculate circularity of the shape (4π * Area / Perimeter²)
/// 1.0 for a perfect circle, < 1.0 for other shapes
pub fn calculate_circularity(area: u32, perimeter: f64) -> f64 {
    if perimeter <= 0.0 {
        return 0.0;
    }
    
    // Initial circularity calculation
    let initial_circularity = (4.0 * PI * area as f64) / (perimeter * perimeter);
    
    // Apply correction for digitization artifacts
    let corrected_perimeter = correct_perimeter(perimeter, initial_circularity);
    
    // Recalculate with corrected perimeter
    (4.0 * PI * area as f64) / (corrected_perimeter * corrected_perimeter)
}

/// Calculate circularity from pre-computed area and contour
pub fn calculate_circularity_from_contour(area: u32, contour: &[(u32, u32)]) -> f64 {
    if contour.len() < 2 {
        return 0.0;
    }
    
    // Apply smoothing to reduce digitization artifacts
    let smoothed_contour = smooth_contour(contour, 3);
    
    // Calculate perimeter from smoothed contour
    let perimeter = calculate_perimeter(&smoothed_contour);
    
    // Calculate circularity with corrections
    calculate_circularity(area, perimeter)
}

/// Analyze shape of the image and return area and circularity
pub fn analyze_shape(image: &RgbaImage, marked_color: [u8; 3]) -> (u32, f64) {
    // Calculate area
    let area = calculate_area(image);
    
    // Trace contour for perimeter calculation
    let raw_contour = trace_contour(image, false, marked_color);
    
    // Calculate circularity from the contour
    let circularity = calculate_circularity_from_contour(area, &raw_contour);
    
    (area, circularity)
}

/// OPTIMIZED: Comprehensive shape analysis with biological dimensions and outline count
/// Returns (area, circularity, length, width, outline_count)
/// Uses biological length/width instead of axis-aligned bounding box
/// This function avoids redundant contour tracing by doing it only once
pub fn analyze_shape_comprehensive(image: &RgbaImage, marked_color: [u8; 3]) -> (u32, f64, f64, f64, u32) {
    // Calculate area (fast - single pass through pixels)
    let area = calculate_area(image);
    
    // Trace contour ONLY ONCE (expensive operation)
    let raw_contour = trace_contour(image, true, marked_color); // Use true for pink as opaque
    
    // Calculate biological dimensions from the pre-computed contour
    let (length, width) = calculate_biological_dimensions_fast(&raw_contour);
    
    // Calculate circularity from the pre-computed contour
    let circularity = calculate_circularity_from_contour(area, &raw_contour);
    
    // Calculate outline count from the pre-computed contour
    let outline_count = calculate_outline_count_from_contour(&raw_contour);
    
    (area, circularity, length, width, outline_count)
}