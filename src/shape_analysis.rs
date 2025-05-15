use image::RgbaImage;
use crate::image_utils::is_non_transparent;
use crate::morphology::trace_contour;
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

/// Calculate circularity of the shape (4π * Area / Perimeter²)
/// 1.0 for a perfect circle, < 1.0 for other shapes
pub fn calculate_circularity(area: u32, perimeter: f64) -> f64 {
    if perimeter <= 0.0 {
        return 0.0;
    }
    
    // Circularity formula: 4π * Area / Perimeter²
    // Normalized to be 1.0 for a perfect circle
    (4.0 * PI * area as f64) / (perimeter * perimeter)
}

/// Analyze shape of the image and return area and circularity
pub fn analyze_shape(image: &RgbaImage, marked_color: [u8; 3]) -> (u32, f64) {
    // Calculate area
    let area = calculate_area(image);
    
    // Trace contour for perimeter calculation
    let contour = trace_contour(image, false, marked_color);
    
    // Calculate perimeter
    let perimeter = calculate_perimeter(&contour);
    
    // Calculate circularity
    let circularity = calculate_circularity(area, perimeter);
    
    (area, circularity)
}