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

/// Analyze shape of the image and return area and circularity
pub fn analyze_shape(image: &RgbaImage, marked_color: [u8; 3]) -> (u32, f64) {
    // Calculate area
    let area = calculate_area(image);
    
    // Trace contour for perimeter calculation
    let raw_contour = trace_contour(image, false, marked_color);
    
    // Apply smoothing to reduce digitization artifacts
    let smoothed_contour = smooth_contour(&raw_contour, 3); // Apply moderate smoothing
    
    // Calculate perimeter from smoothed contour
    let perimeter = calculate_perimeter(&smoothed_contour);
    
    // Calculate circularity with corrections
    let circularity = calculate_circularity(area, perimeter);
    
    (area, circularity)
}