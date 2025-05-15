use image::{ImageBuffer, Rgba, RgbaImage};
use std::cmp::{max, min};
use rayon::prelude::*;

use crate::errors::{LeafComplexError, Result};
use crate::image_utils::{create_circular_kernel, in_bounds, has_rgb_color, ALPHA_THRESHOLD};

/// Applies morphological erosion to the alpha channel
pub fn erode_alpha(
    image: &RgbaImage,
    kernel: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> RgbaImage {
    let (width, height) = image.dimensions();
    let (k_width, k_height) = kernel.dimensions();
    let k_radius_x = (k_width / 2) as i32;
    let k_radius_y = (k_height / 2) as i32;
    
    let mut result = RgbaImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let mut min_alpha = 255u8;
            let mut erode = false;
            
            // Check if any kernel pixel is outside the object
            for ky in 0..k_height {
                for kx in 0..k_width {
                    if kernel.get_pixel(kx, ky)[0] > 0 {
                        let img_x = x as i32 + (kx as i32) - k_radius_x;
                        let img_y = y as i32 + (ky as i32) - k_radius_y;
                        
                        if !in_bounds(img_x, img_y, width, height) {
                            // Consider out-of-bounds as transparent
                            min_alpha = 0;
                            erode = true;
                            break;
                        }
                        
                        let img_alpha = image.get_pixel(img_x as u32, img_y as u32)[3];
                        min_alpha = min(min_alpha, img_alpha);
                        
                        if img_alpha < ALPHA_THRESHOLD {
                            erode = true;
                            break;
                        }
                    }
                }
                if erode {
                    break;
                }
            }
            
            // Copy RGB from original, but use eroded alpha
            let original = image.get_pixel(x, y);
            result.put_pixel(
                x, 
                y, 
                Rgba([original[0], original[1], original[2], if erode { 0 } else { original[3] }])
            );
        }
    }
    
    result
}

/// Applies morphological dilation to the alpha channel
pub fn dilate_alpha(
    image: &RgbaImage,
    kernel: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> RgbaImage {
    let (width, height) = image.dimensions();
    let (k_width, k_height) = kernel.dimensions();
    let k_radius_x = (k_width / 2) as i32;
    let k_radius_y = (k_height / 2) as i32;
    
    let mut result = RgbaImage::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let mut max_alpha = 0u8;
            let mut dilate = false;
            
            // Check if any kernel pixel overlaps with a non-transparent pixel
            for ky in 0..k_height {
                for kx in 0..k_width {
                    if kernel.get_pixel(kx, ky)[0] > 0 {
                        let img_x = x as i32 + (kx as i32) - k_radius_x;
                        let img_y = y as i32 + (ky as i32) - k_radius_y;
                        
                        if in_bounds(img_x, img_y, width, height) {
                            let img_alpha = image.get_pixel(img_x as u32, img_y as u32)[3];
                            max_alpha = max(max_alpha, img_alpha);
                            
                            if img_alpha >= ALPHA_THRESHOLD {
                                dilate = true;
                                break;
                            }
                        }
                    }
                }
                if dilate {
                    break;
                }
            }
            
            // Copy RGB from original, but use dilated alpha
            let original = image.get_pixel(x, y);
            result.put_pixel(
                x, 
                y, 
                Rgba([original[0], original[1], original[2], if dilate { original[3].max(1) } else { original[3] }])
            );
        }
    }
    
    result
}

/// Apply morphological opening (erosion followed by dilation)
pub fn apply_opening(
    image: &RgbaImage, 
    kernel_size: u32
) -> Result<RgbaImage> {
    if kernel_size == 0 {
        return Err(LeafComplexError::Morphology(
            "Kernel size must be greater than 0".to_string()
        ));
    }
    
    // Create circular kernel once
    let kernel = create_circular_kernel(kernel_size);
    
    // Pre-compute kernel properties
    let (k_width, k_height) = kernel.dimensions();
    let k_radius_x = (k_width / 2) as i32;
    let k_radius_y = (k_height / 2) as i32;
    
    // Create kernel lookup for faster access
    let mut kernel_pixels = vec![false; (k_width * k_height) as usize];
    for ky in 0..k_height {
        for kx in 0..k_width {
            if kernel.get_pixel(kx, ky)[0] > 0 {
                kernel_pixels[(ky * k_width + kx) as usize] = true;
            }
        }
    }
    
    // Image properties
    let (width, height) = image.dimensions();
    
    // Apply erosion - using a non-parallel implementation first to fix the issues
    let mut eroded = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let original = image.get_pixel(x, y);
            
            // Skip transparent pixels - they stay transparent
            if original[3] < ALPHA_THRESHOLD {
                eroded.put_pixel(x, y, *original);
                continue;
            }
            
            let mut erode = false;
            // Check kernel
            'kernel_check: for ky in 0..k_height {
                for kx in 0..k_width {
                    if kernel_pixels[(ky * k_width + kx) as usize] {
                        let img_x = x as i32 + (kx as i32) - k_radius_x;
                        let img_y = y as i32 + (ky as i32) - k_radius_y;
                        
                        if img_x < 0 || img_y < 0 || img_x >= width as i32 || img_y >= height as i32 {
                            erode = true;
                            break 'kernel_check;
                        }
                        
                        let img_alpha = image.get_pixel(img_x as u32, img_y as u32)[3];
                        if img_alpha < ALPHA_THRESHOLD {
                            erode = true;
                            break 'kernel_check;
                        }
                    }
                }
            }
            
            let new_pixel = if erode {
                Rgba([original[0], original[1], original[2], 0])
            } else {
                *original
            };
            eroded.put_pixel(x, y, new_pixel);
        }
    }
    
    // Apply dilation
    let mut dilated = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let original = eroded.get_pixel(x, y);
            
            let mut dilate = false;
            // Check kernel
            'kernel_check: for ky in 0..k_height {
                for kx in 0..k_width {
                    if kernel_pixels[(ky * k_width + kx) as usize] {
                        let img_x = x as i32 + (kx as i32) - k_radius_x;
                        let img_y = y as i32 + (ky as i32) - k_radius_y;
                        
                        if img_x >= 0 && img_y >= 0 && img_x < width as i32 && img_y < height as i32 {
                            let img_alpha = eroded.get_pixel(img_x as u32, img_y as u32)[3];
                            if img_alpha >= ALPHA_THRESHOLD {
                                dilate = true;
                                break 'kernel_check;
                            }
                        }
                    }
                }
            }
            
            let new_pixel = if dilate {
                Rgba([original[0], original[1], original[2], original[3].max(1)])
            } else {
                *original
            };
            dilated.put_pixel(x, y, new_pixel);
        }
    }
    
    Ok(dilated)
}

/// Create a marked image where opened regions are colored
pub fn mark_opened_regions(
    original: &RgbaImage, 
    opened: &RgbaImage, 
    color: [u8; 3]
) -> RgbaImage {
    let (width, height) = original.dimensions();
    let mut marked = original.clone();
    
    // First pass: mark pixels that were non-transparent in original but transparent in opened
    for y in 0..height {
        for x in 0..width {
            let orig_pixel = original.get_pixel(x, y);
            let opened_pixel = opened.get_pixel(x, y);
            
            // If pixel was originally non-transparent but is transparent after opening
            if orig_pixel[3] > 0 && opened_pixel[3] == 0 {
                // Mark it with the specified color
                marked.put_pixel(x, y, Rgba([color[0], color[1], color[2], orig_pixel[3]]));
            }
        }
    }
    
    // Second pass: add border detection to mark any single-pixel border
    for y in 0..height {
        for x in 0..width {
            let pixel = marked.get_pixel(x, y);
            
            // Only process non-transparent pixels that aren't already marked
            if pixel[3] > 0 && !has_rgb_color(pixel, color) {
                // Check if this is a border pixel by looking at its neighbors
                let mut is_border = false;
                
                // Check 8-connected neighbors
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; } // Skip the pixel itself
                        
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        
                        // If neighbor is outside the image or transparent
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            is_border = true;
                            break;
                        } else {
                            let neighbor = marked.get_pixel(nx as u32, ny as u32);
                            if neighbor[3] == 0 {
                                is_border = true;
                                break;
                            }
                        }
                    }
                    if is_border { break; }
                }
                
                // If this is a border pixel, mark it
                if is_border {
                    marked.put_pixel(x, y, Rgba([color[0], color[1], color[2], pixel[3]]));
                }
            }
        }
    }
    
    marked
}

/// Direction vectors for Moore-Neighbor contour tracing
static MOORE_NEIGHBORHOOD: [(i32, i32); 8] = [
    (1, 0),   // right
    (1, 1),   // down-right
    (0, 1),   // down
    (-1, 1),  // down-left
    (-1, 0),  // left
    (-1, -1), // up-left
    (0, -1),  // up
    (1, -1),  // up-right
];

/// Trace the contour of a region using Moore-Neighbor tracing algorithm
pub fn trace_contour(image: &RgbaImage, is_pink_opaque: bool, pink_color: [u8; 3]) -> Vec<(u32, u32)> {
    use std::collections::HashSet;
    
    let (width, height) = image.dimensions();
    let mut contour = Vec::new();
    
    // Find the leftmost non-transparent pixel (first pixel encountered in scanning order)
    let mut start_point = None;
    
    'outer: for x in 0..width {
        for y in 0..height {
            let pixel = image.get_pixel(x, y);
            
            // Check if this is a valid contour pixel based on mode
            let is_valid = if is_pink_opaque {
                pixel[3] > 0  // For LEC, any non-transparent pixel is valid
            } else {
                pixel[3] > 0 && !has_rgb_color(pixel, pink_color)  // For LMC, non-transparent and non-pink
            };
            
            if is_valid {
                // Check if it's a boundary pixel by checking if it has any transparent/invalid neighbor
                let mut is_boundary = false;
                
                // Check 8-connected neighbors
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            // Out of bounds counts as boundary
                            is_boundary = true;
                            break;
                        } else {
                            let neighbor = image.get_pixel(nx as u32, ny as u32);
                            let is_neighbor_valid = if is_pink_opaque {
                                neighbor[3] > 0
                            } else {
                                neighbor[3] > 0 && !has_rgb_color(neighbor, pink_color)
                            };
                            
                            if !is_neighbor_valid {
                                is_boundary = true;
                                break;
                            }
                        }
                    }
                    if is_boundary { break; }
                }
                
                if is_boundary {
                    start_point = Some((x, y));
                    break 'outer;
                }
            }
        }
    }
    
    // If no start point found, return empty contour
    let start = match start_point {
        Some(p) => p,
        None => return Vec::new(),
    };
    
    // Add start point to contour
    contour.push(start);
    
    // Initialize variables for contour tracing
    let mut current = start;
    let mut visited = HashSet::new();
    visited.insert(current);
    
    // Direction offsets for 8-connected neighbors (clockwise order from right)
    let directions = [
        (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)
    ];
    
    // Start with the right direction
    let mut direction_idx = 0;
    
    // Safety counter to prevent infinite loops
    let max_iterations = (width * height) as usize * 2;
    let mut iteration_count = 0;
    
    // Main tracing loop
    loop {
        iteration_count += 1;
        if iteration_count > max_iterations {
            println!("Warning: Contour tracing terminated after {} iterations to prevent infinite loop.", max_iterations);
            break;
        }
        
        // Find the next boundary pixel by checking neighbors in clockwise order
        let mut found_next = false;
        
        // Look in all 8 directions, starting from the backtracking direction + 2
        // This ensures we follow the boundary by making a right turn whenever possible
        for i in 0..8 {
            // Start from backtracking direction + 2 (90 degrees clockwise from backtrack)
            let check_idx = (direction_idx + 6 + i) % 8;
            let (dx, dy) = directions[check_idx];
            let nx = current.0 as i32 + dx;
            let ny = current.1 as i32 + dy;
            
            // Check bounds
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            
            let next = (nx as u32, ny as u32);
            let pixel = image.get_pixel(next.0, next.1);
            
            // Check if this pixel is valid according to our criteria
            let is_valid = if is_pink_opaque {
                pixel[3] > 0
            } else {
                pixel[3] > 0 && !has_rgb_color(pixel, pink_color)
            };
            
            if is_valid {
                // We found the next boundary pixel
                current = next;
                direction_idx = check_idx;
                
                // Only add to contour if we haven't visited this pixel yet
                if !visited.contains(&current) {
                    contour.push(current);
                    visited.insert(current);
                }
                
                found_next = true;
                break;
            }
        }
        
        // If we couldn't find a next pixel or we've returned to start and completed at least one circuit
        if !found_next || (current == start && contour.len() > 1) {
            break;
        }
    }
    
    // Return the traced contour
    contour
}

pub fn create_lmc_with_com_component(
    processed_image: &RgbaImage, 
    marked_image: &mut RgbaImage, 
    pink_color: [u8; 3]
) -> RgbaImage {
    let (width, height) = processed_image.dimensions();
    
    // First, calculate the center of mass
    let (com_x, com_y) = calculate_center_of_mass(processed_image)
        .unwrap_or((width / 2, height / 2)); // Fallback to center if calculation fails
    
    println!("Center of Mass: ({}, {})", com_x, com_y);
    
    // Create a version of the image with pink pixels made transparent
    let mut temp_image = marked_image.clone();
    for y in 0..height {
        for x in 0..width {
            let pixel = temp_image.get_pixel_mut(x, y);
            if has_rgb_color(pixel, pink_color) {
                *pixel = image::Rgba([0, 0, 0, 0]);
            }
        }
    }
    
    // Create result image (initially all transparent)
    let mut lmc_image = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            lmc_image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
        }
    }
    
    // Flood fill from COM to find the connected component
    let mut visited = vec![false; (width * height) as usize];
    let mut queue = std::collections::VecDeque::new();
    let mut com_component = Vec::new();
    
    // Start from COM
    queue.push_back((com_x, com_y));
    visited[(com_y * width + com_x) as usize] = true;
    
    // 8-connected neighbors
    let directions = [
        (0, 1), (1, 0), (0, -1), (-1, 0), (1, 1), (1, -1), (-1, 1), (-1, -1)
    ];
    
    // Flood fill
    while let Some((cx, cy)) = queue.pop_front() {
        let idx = (cy * width + cx) as usize;
        
        // Add to component
        com_component.push((cx, cy));
        
        // Copy pixel to LMC image
        lmc_image.put_pixel(cx, cy, *temp_image.get_pixel(cx, cy));
        
        // Check neighbors
        for &(dx, dy) in &directions {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            
            let nx = nx as u32;
            let ny = ny as u32;
            let nidx = (ny * width + nx) as usize;
            
            if visited[nidx] {
                continue;
            }
            
            let npixel = temp_image.get_pixel(nx, ny);
            if npixel[3] > 0 {
                // Non-transparent pixel
                queue.push_back((nx, ny));
                visited[nidx] = true;
            }
        }
    }
    
    println!("COM component has {} pixels", com_component.len());
    
    // Update marking: If pixel is in original but not in COM component, mark it pink
    for y in 0..height {
        for x in 0..width {
            let orig_pixel = processed_image.get_pixel(x, y);
            let lmc_pixel = lmc_image.get_pixel(x, y);
            
            // If pixel is non-transparent in original but transparent in LMC
            if orig_pixel[3] > 0 && lmc_pixel[3] == 0 {
                // Mark it pink in the marked image
                marked_image.put_pixel(x, y, image::Rgba([pink_color[0], pink_color[1], pink_color[2], orig_pixel[3]]));
            }
        }
    }
    
    lmc_image
}

/// Calculate the Center of Mass (COM) - adapted from your existing code
pub fn calculate_center_of_mass(image: &RgbaImage) -> Option<(u32, u32)> {
    let (width, height) = image.dimensions();
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_alpha = 0.0;
    
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let alpha = pixel[3] as f32;
            
            if alpha > 0.0 {
                sum_x += x as f32 * alpha;
                sum_y += y as f32 * alpha;
                sum_alpha += alpha;
            }
        }
    }
    
    if sum_alpha <= 0.0 {
        return None;
    }
    
    // Calculate COM
    let com_x = sum_x / sum_alpha;
    let com_y = sum_y / sum_alpha;
    
    // Round to nearest integer
    Some((com_x.round() as u32, com_y.round() as u32))
}