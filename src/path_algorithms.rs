// src/path_algorithms.rs - Path analysis algorithms for geodesic calculations

use image::RgbaImage;
use bresenham::Bresenham;
use std::collections::{VecDeque, HashMap};

/// Trace a straight line path between two points using Bresenham's algorithm
///
/// # Arguments
/// * `start` - Starting point coordinates
/// * `end` - Ending point coordinates
///
/// # Returns
/// Vector of pixel coordinates along the straight line
pub fn trace_straight_line(
    start: (u32, u32),
    end: (u32, u32),
) -> Vec<(u32, u32)> {
    let (start_x, start_y) = (start.0 as isize, start.1 as isize);
    let (end_x, end_y) = (end.0 as isize, end.1 as isize);
    
    // Use Bresenham's algorithm for straight line
    let line = Bresenham::new((start_x, start_y), (end_x, end_y));
    
    // Convert back to u32 coordinates
    line.map(|(x, y)| (x as u32, y as u32)).collect()
}

/// Check if a straight line path crosses any transparent pixels
///
/// # Arguments
/// * `line_points` - Points along the line to check
/// * `image` - Image to check transparency against
///
/// # Returns
/// true if any pixel along the path is transparent (excluding endpoints)
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
///
/// # Arguments
/// * `point1` - First point coordinates
/// * `point2` - Second point coordinates
///
/// # Returns
/// Euclidean distance as f64
pub fn calculate_straight_path_length(
    point1: (u32, u32),
    point2: (u32, u32),
) -> f64 {
    let dx = point1.0 as f64 - point2.0 as f64;
    let dy = point1.1 as f64 - point2.1 as f64;
    
    (dx * dx + dy * dy).sqrt()
}

/// Calculate the geodesic path (Diego path) that stays within the leaf
///
/// Uses BFS to find the shortest path through non-transparent pixels.
/// If a straight line doesn't cross transparency, returns the straight line.
///
/// # Arguments
/// * `reference_point` - Starting point (reference point)
/// * `margin_point` - Target point (marginal/contour point)
/// * `image` - Image to navigate through
///
/// # Returns
/// Vector of pixel coordinates forming the geodesic path
pub fn calculate_diego_path(
    reference_point: (u32, u32),
    margin_point: (u32, u32),
    image: &RgbaImage
) -> Vec<(u32, u32)> {
    // First, check if the straight line path crosses transparency
    let straight_line = trace_straight_line(reference_point, margin_point);
    
    if !check_straight_line_transparency(&straight_line, image) {
        // No transparency issues, use straight line
        return straight_line;
    }
    
    // Find the last non-transparent point on the straight line
    let mut path = Vec::new();
    
    for &point in &straight_line {
        let pixel = image.get_pixel(point.0, point.1);
        if pixel[3] == 0 {
            break;
        }
        path.push(point);
    }
    
    // If we somehow couldn't find any valid points, return the original straight line
    if path.is_empty() {
        return straight_line;
    }
    
    // Get the starting point for our BFS
    let start_point = path[path.len() - 1];
    
    // BFS to find the shortest path to the margin point
    let (width, height) = image.dimensions();
    let mut queue = VecDeque::new();
    let mut visited = HashMap::new(); // maps point -> previous point for path reconstruction
    
    // Start the BFS
    queue.push_back(start_point);
    visited.insert(start_point, start_point); // mark start as visited, pointing to itself
    
    // The 8 adjacent directions (cardinal directions first for preference)
    let directions = [
        (0, 1), (1, 0), (0, -1), (-1, 0),  // Cardinal
        (1, 1), (1, -1), (-1, 1), (-1, -1) // Diagonal
    ];
    
    let mut target_found = false;
    let max_iterations = (width * height) as usize * 2;
    let mut iteration_count = 0;
    
    while !queue.is_empty() && !target_found {
        iteration_count += 1;
        if iteration_count > max_iterations {
            println!("Warning: Geodesic path search terminated after {} iterations", max_iterations);
            break;
        }
        
        let current = queue.pop_front().unwrap();
        
        // Check each adjacent pixel
        for &(dx, dy) in &directions {
            let nx = current.0 as i32 + dx;
            let ny = current.1 as i32 + dy;
            
            // Check bounds
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            
            let next = (nx as u32, ny as u32);
            
            // Skip if already visited
            if visited.contains_key(&next) {
                continue;
            }
            
            // Skip transparent pixels
            let pixel = image.get_pixel(next.0, next.1);
            if pixel[3] == 0 {
                continue;
            }
            
            // Mark as visited and remember how we got here
            visited.insert(next, current);
            
            // Check if we've reached the target
            if next == margin_point {
                target_found = true;
                break;
            }
            
            // Add to queue to explore later
            queue.push_back(next);
        }
    }
    
    // If we found a path to the target, reconstruct it
    if target_found {
        // Reconstruct the path backwards from target to start
        let mut backpath = Vec::new();
        let mut current = margin_point;
        
        while current != start_point {
            backpath.push(current);
            current = *visited.get(&current).unwrap();
        }
        
        // Reverse the backpath and add it to our original path
        for &point in backpath.iter().rev() {
            path.push(point);
        }
        
        return path;
    }
    
    // If we didn't find a path with BFS, return what we have
    println!("BFS couldn't find a path to target");
    path
}

/// Calculate the path length of the Diego (geodesic) path
///
/// # Arguments
/// * `path` - Vector of pixel coordinates forming the path
///
/// # Returns
/// Total length of the path in pixels
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

/// Calculate number of marked pixels (pink) along the Diego path
///
/// Used for EC (Edge Complexity) analysis to count intersections with marked regions.
///
/// # Arguments
/// * `path` - Vector of pixel coordinates forming the path
/// * `marked_image` - Image with marked regions (pink pixels)
/// * `pink_color` - RGB color of the marked regions
///
/// # Returns
/// Count of pink pixels crossed by the path
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
