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

/// Find the external contour of non-transparent regions
/// Find the external contour of non-transparent regions
pub fn trace_contour(image: &RgbaImage, is_pink_opaque: bool, pink_color: [u8; 3]) -> Vec<(u32, u32)> {
    let (width, height) = image.dimensions();
    let mut contour = Vec::new();
    
    // Critical fix: Use a visited array to prevent retracing pixels
    let mut visited = vec![vec![false; height as usize]; width as usize];
    
    // Find the first contour point (scanning from top-left)
    let mut start_point = None;
    
    'outer: for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            
            // Check if this is a non-transparent pixel we should include in contour
            let include_in_contour = if is_pink_opaque {
                // For LEC: Both non-transparent and pink regions are considered part of the object
                pixel[3] > 0
            } else {
                // For LMC: Only non-transparent, non-pink regions are part of the object
                pixel[3] > 0 && !has_rgb_color(pixel, pink_color)
            };
            
            if include_in_contour {
                // Check if it's on the border by looking at any of its neighbors
                let mut is_border = false;
                
                // Check 8-connected neighbors
                for &(dx, dy) in &MOORE_NEIGHBORHOOD {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    
                    // If neighbor is outside or transparent/pink (depending on mode)
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        is_border = true;
                        break;
                    } else {
                        let neighbor = image.get_pixel(nx as u32, ny as u32);
                        let neighbor_transparent = neighbor[3] == 0;
                        let neighbor_pink = has_rgb_color(neighbor, pink_color);
                        
                        if (neighbor_transparent) || (!is_pink_opaque && neighbor_pink) {
                            is_border = true;
                            break;
                        }
                    }
                }
                
                if is_border {
                    start_point = Some((x, y));
                    break 'outer;
                }
            }
        }
    }
    
    // If no contour point was found, return empty vector
    let (start_x, start_y) = match start_point {
        Some(point) => point,
        None => return contour,
    };
    
    // Add the start point to the contour
    contour.push((start_x, start_y));
    visited[start_x as usize][start_y as usize] = true;
    
    // Initialize current position and backtrack index
    let mut current = (start_x, start_y);
    let mut jacob_idx = 0; // Start looking from the first Moore neighbor
    
    // Important fix: Add safety limit to prevent infinite loops
    let max_contour_size = 2 * (width + height) as usize; // Reasonable maximum perimeter
    
    // Trace the contour using Moore-Neighbor tracing
    loop {
        let mut found_next = false;
        
        // Search in Moore neighborhood, starting from the backtrack direction
        for i in 0..8 {
            let idx = (jacob_idx + i) % 8;
            let (dx, dy) = MOORE_NEIGHBORHOOD[idx];
            let nx = current.0 as i32 + dx;
            let ny = current.1 as i32 + dy;
            
            if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                let next_x = nx as u32;
                let next_y = ny as u32;
                let pixel = image.get_pixel(next_x, next_y);
                
                // Check if this neighbor should be included in contour
                let include_in_contour = if is_pink_opaque {
                    // For LEC: Both non-transparent and pink regions are considered part of the object
                    pixel[3] > 0
                } else {
                    // For LMC: Only non-transparent, non-pink regions are part of the object
                    pixel[3] > 0 && !has_rgb_color(pixel, pink_color)
                };
                
                // Critical fix: Only add unvisited pixels
                if include_in_contour && !visited[next_x as usize][next_y as usize] {
                    // Add to contour
                    contour.push((next_x, next_y));
                    visited[next_x as usize][next_y as usize] = true;
                    
                    // Update current position and backtrack index
                    current = (next_x, next_y);
                    jacob_idx = (idx + 4) % 8; // Backtrack direction
                    
                    found_next = true;
                    break;
                }
            }
        }
        
        // Safety check: Break if contour is becoming too large
        if contour.len() > max_contour_size {
            println!("Warning: Contour exceeded maximum size ({}), stopping early.", max_contour_size);
            break;
        }
        
        // If we couldn't find the next point or we've returned to the start, we're done
        if !found_next || (current.0 == start_x && current.1 == start_y && contour.len() > 1) {
            break;
        }
    }
    
    // Final safety check: Deduplicate the contour points
    let mut unique_points = Vec::new();
    let mut point_set = std::collections::HashSet::new();
    
    for point in contour {
        let point_key = (point.0 as u64) << 32 | (point.1 as u64);
        if point_set.insert(point_key) {
            unique_points.push(point);
        }
    }
    
    unique_points
}