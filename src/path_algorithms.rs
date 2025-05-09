use bresenham::Bresenham;
use image::RgbaImage;

/// Constant for the golden ratio
pub const PHI: f64 = 1.618033988749895; // (1.0 + 5.0_f64.sqrt()) / 2.0

/// Trace a straight line path between two points
pub fn trace_straight_line(
    start: (u32, u32),
    end: (u32, u32),
) -> Vec<(u32, u32)> {
    // Convert to isize for Bresenham
    let (start_x, start_y) = (start.0 as isize, start.1 as isize);
    let (end_x, end_y) = (end.0 as isize, end.1 as isize);
    
    // Use Bresenham's algorithm for straight line
    let line = Bresenham::new((start_x, start_y), (end_x, end_y));
    
    // Convert back to u32 coordinates
    line.map(|(x, y)| (x as u32, y as u32)).collect()
}

/// Check if a straight line path crosses any transparent pixels
pub fn check_straight_line_transparency(
    line_points: &[(u32, u32)],
    image: &RgbaImage,
) -> bool {
    let (width, height) = image.dimensions();
    
    // Skip start and end points in the check
    if line_points.len() <= 2 {
        return false;
    }
    
    for i in 1..(line_points.len() - 1) {
        let (x, y) = line_points[i];
        
        if x < width && y < height {
            let pixel = image.get_pixel(x, y);
            
            // If any pixel along the path is transparent, return true
            if pixel[3] == 0 {
                return true;
            }
        }
    }
    
    false
}

/// Calculate the Euclidean distance between two points
pub fn calculate_straight_path_length(
    point1: (u32, u32),
    point2: (u32, u32),
) -> f64 {
    let dx = point1.0 as f64 - point2.0 as f64;
    let dy = point1.1 as f64 - point2.1 as f64;
    
    (dx * dx + dy * dy).sqrt()
}

/// Calculate the angle between two points in radians
pub fn calculate_angle(
    center: (u32, u32),
    point: (u32, u32),
) -> f64 {
    let dx = point.0 as f64 - center.0 as f64;
    let dy = point.1 as f64 - center.1 as f64;
    
    dy.atan2(dx)
}

/// Calculate golden spiral parameters based on straight path length
pub fn calculate_golden_spiral_params(straight_path_length: f64, _phi_exponent_factor: f64) -> (f64, f64) {
    // Calculate the total width of the golden spiral (sum of first two squares)
    // This will be the straight path length
    let total_width = straight_path_length;
    
    // Calculate the size of the first square (largest)
    // Using the golden ratio: a + b = total_width, where a/b = φ
    // Therefore: a = total_width * φ/(1 + φ)
    let phi = (1.0 + (5.0f64).sqrt()) / 2.0;
    let first_square_size = total_width * phi / (1.0 + phi);
    
    // Calculate the size of the second square
    let second_square_size = total_width - first_square_size;
    
    // Calculate the sizes of the remaining squares (following Fibonacci sequence)
    let mut square_sizes = vec![first_square_size, second_square_size];
    for i in 2..5 {
        square_sizes.push(square_sizes[i-2] - square_sizes[i-1]);
    }
    
    // The spiral coefficient 'a' is the size of the first square
    let spiral_a_coeff = first_square_size;
    
    // Calculate theta_contact based on the number of rotations needed
    // We want to complete 5 squares, which requires approximately 2.5 rotations
    let theta_contact = 5.0 * std::f64::consts::PI;
    
    (spiral_a_coeff, theta_contact)
}

/// Generate a golden spiral path
pub fn generate_golden_spiral_path(
    start_point: (u32, u32),
    end_point: (u32, u32),
    spiral_a_coeff: f64,
    theta_contact: f64,
    _phi_exponent_factor: f64,
    num_points: usize,
) -> Vec<(u32, u32)> {
    let mut path = Vec::with_capacity(num_points);
    
    // Calculate the angle between start and end points
    let dx = end_point.0 as f64 - start_point.0 as f64;
    let dy = end_point.1 as f64 - start_point.1 as f64;
    let base_angle = dy.atan2(dx);
    
    // Calculate the golden ratio
    let phi = (1.0 + (5.0f64).sqrt()) / 2.0;
    
    // Generate points along the spiral
    for i in 0..num_points {
        let t = (i as f64) / (num_points - 1) as f64;
        let theta = t * theta_contact;
        
        // Calculate radius using the golden spiral formula
        let radius = spiral_a_coeff * (phi.powf(theta / (2.0 * std::f64::consts::PI)));
        
        // Calculate point coordinates
        let x = start_point.0 as f64 + radius * (base_angle + theta).cos();
        let y = start_point.1 as f64 + radius * (base_angle + theta).sin();
        
        path.push((x.round() as u32, y.round() as u32));
    }
    
    path
}

/// Check if a Golden Spiral path is valid (all points are non-transparent)
pub fn check_spiral_path_validity(
    path: &[(u32, u32)],
    image: &RgbaImage,
) -> bool {
    let (width, height) = image.dimensions();
    
    for &(x, y) in path {
        if x < width && y < height {
            let pixel = image.get_pixel(x, y);
            
            // If any pixel along the path is transparent, the path is invalid
            if pixel[3] == 0 {
                return false;
            }
        } else {
            // If any point is outside image bounds, the path is invalid
            return false;
        }
    }
    
    true
}

/// Calculate the arc length of the Golden Spiral segment
pub fn calculate_gyro_path_length(
    spiral_a_coeff: f64,
    theta_contact: f64,
    phi_exponent_factor: f64,
) -> f64 {
    // The arc length formula for a logarithmic spiral from 0 to theta is:
    // L = (a / sqrt(1 + b^2 * ln(φ)^2)) * (φ^(b*theta) - 1)
    // where b = phi_exponent_factor
    
    let b = phi_exponent_factor;
    let b_ln_phi = b * PHI.ln();
    let term1 = spiral_a_coeff / (1.0 + b_ln_phi * b_ln_phi).sqrt();
    let term2 = PHI.powf(b * theta_contact) - 1.0;
    
    term1 * term2
}

/// Calculate CLR_Alpha and CLR_Gamma points along a spiral path
pub fn calculate_clr_points(
    reference_point: (u32, u32),
    marginal_point: (u32, u32),
    spiral_path: &[(u32, u32)],
    image: &RgbaImage,
) -> (u32, u32) {
    let mut alpha = 0;
    let mut gamma = 0;
    
    // Create polygon from straight line and spiral path
    let mut polygon = Vec::new();
    let straight_line = trace_straight_line(reference_point, marginal_point);
    polygon.extend_from_slice(&straight_line);
    
    // Reverse spiral path for proper polygon formation
    let mut spiral_path_rev = spiral_path.to_vec();
    spiral_path_rev.reverse();
    polygon.extend_from_slice(&spiral_path_rev);
    
    // Calculate bounding box (with padding)
    let padding = 10;
    let min_x = reference_point.0.min(marginal_point.0).saturating_sub(padding);
    let max_x = reference_point.0.max(marginal_point.0) + padding;
    let min_y = reference_point.1.min(marginal_point.1).saturating_sub(padding);
    let max_y = reference_point.1.max(marginal_point.1) + padding;
    
    // Expand bounding box to include spiral path
    let expanded_bbox = spiral_path.iter().fold((min_x, min_y, max_x, max_y), |acc, &(x, y)| {
        (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y))
    });
    
    let (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y) = expanded_bbox;
    
    // Count pixels in each category
    let width = image.width();
    let height = image.height();
    
    for y in bbox_min_y..=bbox_max_y {
        if y >= height {
            continue;
        }
        
        for x in bbox_min_x..=bbox_max_x {
            if x >= width {
                continue;
            }
            
            // Check if the point is inside the polygon
            if is_point_in_polygon(x as f32, y as f32, &polygon) {
                let pixel = image.get_pixel(x, y);
                
                // Check if transparent
                if pixel[3] == 0 {
                    alpha += 1;
                } else {
                    gamma += 1;
                }
            }
        }
    }
    
    (alpha, gamma)
}

/// Calculate the number of pink pixels along a spiral path
pub fn calculate_gyro_path_pink(
    spiral_path: &[(u32, u32)],
    marked_image: &RgbaImage,
    marked_color: [u8; 3],
) -> u32 {
    let mut pink_count = 0;
    
    for &point in spiral_path {
        let (width, height) = marked_image.dimensions();
        if point.0 < width && point.1 < height {
            let pixel = marked_image.get_pixel(point.0, point.1);
            
            // Check if pixel matches the marked color (pink)
            if pixel[0] == marked_color[0] && 
               pixel[1] == marked_color[1] && 
               pixel[2] == marked_color[2] {
                pink_count += 1;
            }
        }
    }
    
    pink_count
}

/// Check if a point is inside a polygon (ray casting algorithm)
pub fn is_point_in_polygon(px: f32, py: f32, polygon: &[(u32, u32)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    
    let mut inside = false;
    let mut j = polygon.len() - 1;
    
    for i in 0..polygon.len() {
        let (xi, yi) = (polygon[i].0 as f32, polygon[i].1 as f32);
        let (xj, yj) = (polygon[j].0 as f32, polygon[j].1 as f32);
        
        let intersect = ((yi > py) != (yj > py)) && 
                        (px < xi + (py - yi) * (xj - xi) / (yj - yi));
        
        if intersect {
            inside = !inside;
        }
        
        j = i;
    }
    
    inside
}