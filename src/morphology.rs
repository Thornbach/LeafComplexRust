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
pub fn trace_contour(image: &RgbaImage, is_pink_opaque: bool, pink_color: [u8; 3]) -> Vec<(u32, u32)> {
    let (width, height) = image.dimensions();
    let mut contour = Vec::new();
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
                // Check if it's on the border by looking at its right neighbor
                if x + 1 >= width || {
                    let right_pixel = image.get_pixel(x + 1, y);
                    let right_is_transparent = right_pixel[3] == 0;
                    let right_is_pink = has_rgb_color(right_pixel, pink_color);
                    
                    if is_pink_opaque {
                        // For LEC: Include in contour if right pixel is transparent
                        right_is_transparent
                    } else {
                        // For LMC: Include in contour if right pixel is transparent or pink
                        right_is_transparent || right_is_pink
                    }
                } {
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
        
        // If we couldn't find the next point or we've returned to the start, we're done
        if !found_next || (current.0 == start_x && current.1 == start_y && contour.len() > 1) {
            break;
        }
    }
    
    contour
}