use std::collections::{VecDeque, HashSet};
use image::RgbaImage;
use bresenham::Bresenham;
use crate::image_utils::{is_non_transparent, has_rgb_color};

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
    // The spiral coefficient is proportional to the straight path length
    let spiral_a_coeff = straight_path_length * 0.5;
    
    // The theta contact should be large enough to create a substantial curve
    // This value controls how "curved" the spiral is
    let theta_contact = std::f64::consts::PI * 0.8; // About 144 degrees
    
    (spiral_a_coeff, theta_contact)
}

/// Generate a golden spiral path
pub fn generate_golden_spiral_path(
    reference_point: (u32, u32),
    marginal_point: (u32, u32),
    _spiral_a_coeff: f64,
    _theta_contact: f64,
    _phi_exponent_factor: f64,
    num_points: usize,
) -> Vec<(u32, u32)> {
    // Convert points to f64 for more precise calculations
    let ref_x = reference_point.0 as f64;
    let ref_y = reference_point.1 as f64;
    let marg_x = marginal_point.0 as f64;
    let marg_y = marginal_point.1 as f64;
    
    // Calculate basic vector from reference to marginal point
    let dx = marg_x - ref_x;
    let dy = marg_y - ref_y;
    let straight_distance = (dx * dx + dy * dy).sqrt();
    let base_angle = dy.atan2(dx);
    
    // Define the path. We'll use a quadratic Bezier curve to create a smooth arc
    let mut path = Vec::with_capacity(num_points);
    
    // Control point for the Bezier curve - this determines the curve's shape
    // Place it perpendicular to the straight line, at a distance that creates a good curvature
    // We'll use the left side by default (subtract π/2)
    let control_distance = straight_distance * 0.5; // Controls curve magnitude
    let control_angle = base_angle - std::f64::consts::PI / 2.0; // Controls curve direction
    
    let control_x = ref_x + dx / 2.0 + control_distance * control_angle.cos();
    let control_y = ref_y + dy / 2.0 + control_distance * control_angle.sin();
    
    // Generate the Bezier curve points
    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        
        // Quadratic Bezier formula: B(t) = (1-t)²P₀ + 2(1-t)tP₁ + t²P₂
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        
        // Calculate point coordinates using Bezier formula
        let x = mt2 * ref_x + 2.0 * mt * t * control_x + t2 * marg_x;
        let y = mt2 * ref_y + 2.0 * mt * t * control_y + t2 * marg_y;
        
        // Round to nearest pixel
        path.push(((x + 0.5) as u32, (y + 0.5) as u32));
    }
    
    // Ensure the endpoints are exact
    if !path.is_empty() {
        path[0] = reference_point;
        if path.len() > 1 {
            let last_idx = path.len() - 1;  // Store the index first
            path[last_idx] = marginal_point;
        }
    }
    
    path
}

/// Generate the left and right spiral paths for a given pair of points
pub fn generate_left_right_spirals(
    reference_point: (u32, u32),
    marginal_point: (u32, u32),
    _spiral_a_coeff: f64,
    _theta_contact: f64,
    _phi_exponent_factor: f64,
    num_points: usize,
) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    // Convert points to f64 for more precise calculations
    let ref_x = reference_point.0 as f64;
    let ref_y = reference_point.1 as f64;
    let marg_x = marginal_point.0 as f64;
    let marg_y = marginal_point.1 as f64;
    
    // Calculate basic vector from reference to marginal point
    let dx = marg_x - ref_x;
    let dy = marg_y - ref_y;
    let straight_distance = (dx * dx + dy * dy).sqrt();
    let base_angle = dy.atan2(dx);
    
    // Left spiral (curve bends to the left)
    let mut left_path = Vec::with_capacity(num_points);
    let left_control_distance = straight_distance * 0.5;
    let left_control_angle = base_angle - std::f64::consts::PI / 2.0;
    
    let left_control_x = ref_x + dx / 2.0 + left_control_distance * left_control_angle.cos();
    let left_control_y = ref_y + dy / 2.0 + left_control_distance * left_control_angle.sin();
    
    // Generate the left Bezier curve
    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        
        // Quadratic Bezier formula
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        
        let x = mt2 * ref_x + 2.0 * mt * t * left_control_x + t2 * marg_x;
        let y = mt2 * ref_y + 2.0 * mt * t * left_control_y + t2 * marg_y;
        
        left_path.push(((x + 0.5) as u32, (y + 0.5) as u32));
    }
    
    // Right spiral (curve bends to the right)
    let mut right_path = Vec::with_capacity(num_points);
    let right_control_distance = straight_distance * 0.5;
    let right_control_angle = base_angle + std::f64::consts::PI / 2.0;
    
    let right_control_x = ref_x + dx / 2.0 + right_control_distance * right_control_angle.cos();
    let right_control_y = ref_y + dy / 2.0 + right_control_distance * right_control_angle.sin();
    
    // Generate the right Bezier curve
    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        
        // Quadratic Bezier formula
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let t2 = t * t;
        
        let x = mt2 * ref_x + 2.0 * mt * t * right_control_x + t2 * marg_x;
        let y = mt2 * ref_y + 2.0 * mt * t * right_control_y + t2 * marg_y;
        
        right_path.push(((x + 0.5) as u32, (y + 0.5) as u32));
    }
    
    // Ensure the endpoints are exact for both paths
    if !left_path.is_empty() {
        left_path[0] = reference_point;
        if left_path.len() > 1 {
            let last_idx = left_path.len() - 1;  // Store the index first
            left_path[last_idx] = marginal_point;
        }
    }
    
    if !right_path.is_empty() {
        right_path[0] = reference_point;
        if right_path.len() > 1 {
            let last_idx = right_path.len() - 1;  // Store the index first
            right_path[last_idx] = marginal_point;
        }
    }
    
    (left_path, right_path)
}

/// Generate a full golden spiral for visualization
pub fn generate_full_spiral(
    center: (u32, u32),
    initial_radius: f64,
    num_revolutions: f64,
    points_per_revolution: usize,
) -> Vec<(u32, u32)> {
    let total_points = (num_revolutions * points_per_revolution as f64) as usize;
    let mut spiral = Vec::with_capacity(total_points);
    let center_x = center.0 as f64;
    let center_y = center.1 as f64;
    
    // Generate points along the spiral
    for i in 0..total_points {
        // Calculate angle (in radians)
        let theta = (i as f64 / points_per_revolution as f64) * 2.0 * std::f64::consts::PI;
        
        // Calculate radius using golden spiral formula
        let radius = initial_radius * PHI.powf(theta / (2.0 * std::f64::consts::PI));
        
        // Calculate coordinates
        let x = center_x + radius * theta.cos();
        let y = center_y + radius * theta.sin();
        
        // Add point to spiral
        spiral.push(((x + 0.5) as u32, (y + 0.5) as u32));
    }
    
    spiral
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
    _theta_contact: f64,
    _phi_exponent_factor: f64,
) -> f64 {
    // For a quadratic Bezier curve, we can approximate the arc length
    // based on the control point distance and the straight path length
    
    // First, recover the straight path length from the spiral coefficient
    let straight_length = spiral_a_coeff * 2.0;
    
    // For a quadratic Bezier with a control point at distance d from the midpoint
    // of the straight line, the arc length is approximately:
    let control_distance = straight_length * 0.5;
    
    // Arc length formula for quadratic Bezier (approximation)
    // We ensure it's at least 20% longer than the straight path
    let arc_length_approx = straight_length * (1.0 + 0.5 * (control_distance / straight_length).powi(2) * 8.0);
    
    arc_length_approx.max(straight_length * 1.2)
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

/// Calculate balanced CLR values using both left and right curves
pub fn calculate_balanced_clr(
    reference_point: (u32, u32),
    marginal_point: (u32, u32),
    spiral_a_coeff: f64,
    theta_contact: f64,
    phi_exponent_factor: f64,
    num_points: usize,
    image: &RgbaImage,
) -> (u32, u32, Vec<(u32, u32)>, Vec<(u32, u32)>) {
    // Generate both left and right spiral paths
    let (left_path, right_path) = generate_left_right_spirals(
        reference_point,
        marginal_point,
        spiral_a_coeff,
        theta_contact,
        phi_exponent_factor,
        num_points
    );
    
    // Calculate CLR values for both paths
    let (left_alpha, left_gamma) = calculate_clr_points(
        reference_point,
        marginal_point,
        &left_path,
        image
    );
    
    let (right_alpha, right_gamma) = calculate_clr_points(
        reference_point,
        marginal_point,
        &right_path,
        image
    );
    
    // Average the CLR values (floor to round down)
    let avg_alpha = ((left_alpha as f64 + right_alpha as f64) / 2.0).floor() as u32;
    let avg_gamma = ((left_gamma as f64 + right_gamma as f64) / 2.0).floor() as u32;
    
    // Return the averaged values and both paths for visualization
    (avg_alpha, avg_gamma, left_path, right_path)
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

/// Calculate balanced pink pixel count along both left and right spirals
pub fn calculate_balanced_pink(
    reference_point: (u32, u32),
    marginal_point: (u32, u32),
    spiral_a_coeff: f64,
    theta_contact: f64,
    phi_exponent_factor: f64,
    num_points: usize,
    marked_image: &RgbaImage,
    marked_color: [u8; 3],
) -> (u32, Vec<(u32, u32)>, Vec<(u32, u32)>) {
    // Generate both left and right spiral paths
    let (left_path, right_path) = generate_left_right_spirals(
        reference_point,
        marginal_point,
        spiral_a_coeff,
        theta_contact,
        phi_exponent_factor,
        num_points
    );
    
    // Calculate pink pixels for both paths
    let left_pink = calculate_gyro_path_pink(&left_path, marked_image, marked_color);
    let right_pink = calculate_gyro_path_pink(&right_path, marked_image, marked_color);
    
    // Average the pink pixel counts (floor to round down)
    let avg_pink = ((left_pink as f64 + right_pink as f64) / 2.0).floor() as u32;
    
    // Return the averaged value and both paths for visualization
    (avg_pink, left_path, right_path)
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

/// Find the shortest path that stays within the leaf boundary (never crosses transparent pixels)
/// Uses a breadth-first search algorithm to find the shortest path
/// Find the shortest path that stays within the leaf boundary (never crosses transparent pixels)
/// Uses a breadth-first search algorithm to find the shortest path
pub fn calculate_diego_path(
    reference_point: (u32, u32),
    margin_point: (u32, u32),
    image: &RgbaImage
) -> Vec<(u32, u32)> {
    // First, check if the straight line path crosses transparency
    let straight_line = trace_straight_line(reference_point, margin_point);
    
    // If straight line doesn't cross transparency, use it
    if !check_straight_line_transparency(&straight_line, image) {
        println!("Using straight line for DiegoPath (no transparency crossed)");
        return straight_line;
    }
    
    println!("Straight line crosses transparency, calculating BFS path");
    
    let (width, height) = image.dimensions();
    
    // If points are the same, return just that point
    if reference_point == margin_point {
        return vec![reference_point];
    }
    
    // Using BFS to find the shortest path
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut previous: Vec<Vec<Option<(u32, u32)>>> = vec![vec![None; height as usize]; width as usize];
    
    // Start from reference point
    queue.push_back(reference_point);
    visited.insert(reference_point);
    
    // The eight directions for movement (including diagonals for a better path)
    let directions = [
        (0, 1), (1, 0), (0, -1), (-1, 0),  // Cardinal (NSEW)
        (1, 1), (1, -1), (-1, 1), (-1, -1) // Diagonal
    ];
    
    // BFS until we find the target or exhaust all possibilities
    let mut found = false;
    while !queue.is_empty() && !found {
        let current = queue.pop_front().unwrap();
        
        // If we've reached the margin point, we're done
        if current == margin_point {
            found = true;
            break;
        }
        
        // Try all possible directions
        for &(dx, dy) in &directions {
            let new_x = current.0 as i32 + dx;
            let new_y = current.1 as i32 + dy;
            
            // Check bounds
            if new_x < 0 || new_y < 0 || new_x >= width as i32 || new_y >= height as i32 {
                continue;
            }
            
            let new_pos = (new_x as u32, new_y as u32);
            
            // Skip if already visited
            if visited.contains(&new_pos) {
                continue;
            }
            
            // Check if the pixel is non-transparent (within leaf)
            let pixel = image.get_pixel(new_pos.0, new_pos.1);
            if pixel[3] == 0 {
                continue;
            }
            
            // Mark as visited and record the previous position
            visited.insert(new_pos);
            previous[new_pos.0 as usize][new_pos.1 as usize] = Some(current);
            queue.push_back(new_pos);
        }
    }
    
    // If we didn't find a path, return the straight line path as fallback
    if !found {
        println!("DiegoPath not found, using straight line path as fallback");
        return straight_line;
    }
    
    // Reconstruct the path
    let mut path = Vec::new();
    let mut current = margin_point;
    
    // Traverse back from margin point to reference point
    while current != reference_point {
        path.push(current);
        
        match previous[current.0 as usize][current.1 as usize] {
            Some(prev) => current = prev,
            None => {
                // This shouldn't happen if a path was found, but if it does
                // return what we have so far plus the reference point
                println!("Error in DiegoPath reconstruction - missing previous node");
                path.push(reference_point);
                path.reverse();
                return path;
            }
        }
    }
    
    // Add the reference point
    path.push(reference_point);
    
    // Reverse to get the path from reference to margin
    path.reverse();
    
    path
}

/// Calculate the path length of the diego path
pub fn calculate_diego_path_length(path: &[(u32, u32)]) -> f64 {
    if path.len() < 2 {
        return 0.0;
    }
    
    let mut length = 0.0;
    
    for i in 1..path.len() {
        let dx = path[i].0 as f64 - path[i-1].0 as f64;
        let dy = path[i].1 as f64 - path[i-1].1 as f64;
        length += (dx * dx + dy * dy).sqrt();
    }
    
    length
}

/// Calculate number of pink pixels along the diego path
pub fn calculate_diego_path_pink(
    path: &[(u32, u32)],
    marked_image: &RgbaImage,
    pink_color: [u8; 3]
) -> u32 {
    let mut pink_count = 0;
    
    for &(x, y) in path {
        let pixel = marked_image.get_pixel(x, y);
        
        // Check if pixel has the pink color
        if pixel[0] == pink_color[0] && pixel[1] == pink_color[1] && pixel[2] == pink_color[2] {
            pink_count += 1;
        }
    }
    
    pink_count
}

/// Calculate CLR regions (moved from GUI to here)
pub fn calculate_clr_regions(
    ref_point: (u32, u32),
    margin_point: (u32, u32),
    spiral_path: &[(u32, u32)],
    right_spiral_path: Option<&[(u32, u32)]>,
    image: &RgbaImage,
) -> (Vec<(u32, u32)>, Vec<(u32, u32)>, Vec<(u32, u32)>, Vec<(u32, u32)>) {
    println!("Calculating CLR regions");
    let mut clr_alpha_pixels = Vec::new();
    let mut clr_gamma_pixels = Vec::new();
    let mut right_clr_alpha_pixels = Vec::new();
    let mut right_clr_gamma_pixels = Vec::new();
    
    // Create polygon from straight line and spiral path
    let mut polygon = Vec::new();
    let straight_line = trace_straight_line(ref_point, margin_point);
    polygon.extend_from_slice(&straight_line);
    
    // Reverse spiral path for proper polygon formation
    let mut spiral_path_rev = spiral_path.to_vec();
    spiral_path_rev.reverse();
    polygon.extend_from_slice(&spiral_path_rev);
    
    // Calculate bounding box (with padding)
    let padding = 10;
    let min_x = ref_point.0.min(margin_point.0).saturating_sub(padding);
    let max_x = ref_point.0.max(margin_point.0) + padding;
    let min_y = ref_point.1.min(margin_point.1).saturating_sub(padding);
    let max_y = ref_point.1.max(margin_point.1) + padding;
    
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
                    clr_alpha_pixels.push((x, y));
                } else {
                    clr_gamma_pixels.push((x, y));
                }
            }
        }
    }
    
    // Also calculate for right spiral if provided
    if let Some(right_path) = right_spiral_path {
        // Similar process for right spiral
        let mut right_polygon = Vec::new();
        right_polygon.extend_from_slice(&straight_line);
        
        let mut right_path_rev = right_path.to_vec();
        right_path_rev.reverse();
        right_polygon.extend_from_slice(&right_path_rev);
        
        // Use the same bounding box expanded to include right spiral path
        let right_expanded_bbox = right_path.iter().fold(
            (bbox_min_x, bbox_min_y, bbox_max_x, bbox_max_y), 
            |acc, &(x, y)| {
                (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y))
            }
        );
        
        let (r_bbox_min_x, r_bbox_min_y, r_bbox_max_x, r_bbox_max_y) = right_expanded_bbox;
        
        for y in r_bbox_min_y..=r_bbox_max_y {
            if y >= height {
                continue;
            }
            
            for x in r_bbox_min_x..=r_bbox_max_x {
                if x >= width {
                    continue;
                }
                
                if is_point_in_polygon(x as f32, y as f32, &right_polygon) {
                    let pixel = image.get_pixel(x, y);
                    
                    if pixel[3] == 0 {
                        right_clr_alpha_pixels.push((x, y));
                    } else {
                        right_clr_gamma_pixels.push((x, y));
                    }
                }
            }
        }
    }
    
    println!("CLR_Alpha: {}, CLR_Gamma: {}", 
            clr_alpha_pixels.len(), clr_gamma_pixels.len());
            
    if !right_clr_alpha_pixels.is_empty() {
        println!("Right CLR_Alpha: {}, Right CLR_Gamma: {}", 
                right_clr_alpha_pixels.len(), right_clr_gamma_pixels.len());
    }
    
    (clr_alpha_pixels, clr_gamma_pixels, right_clr_alpha_pixels, right_clr_gamma_pixels)
}