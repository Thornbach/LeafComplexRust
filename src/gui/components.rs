// src/gui/components.rs - UI component drawing

use super::font::FONT_BITMAP; // Updated path
use super::state::{WINDOW_WIDTH, WINDOW_HEIGHT};

/// Draw a circle
pub fn draw_circle(buffer: &mut [u32], center_x: usize, center_y: usize, radius: usize, 
               width: usize, height: usize, color: u32) {
    let radius_sq = (radius * radius) as isize;
    
    for y in center_y.saturating_sub(radius)..std::cmp::min(center_y + radius + 1, height) {
        for x in center_x.saturating_sub(radius)..std::cmp::min(center_x + radius + 1, width) {
            let dx = x as isize - center_x as isize;
            let dy = y as isize - center_y as isize;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq <= radius_sq {
                let idx = y * width + x;
                if idx < buffer.len() {
                    buffer[idx] = color;
                }
            }
        }
    }
}

/// Draw a rectangle
pub fn draw_rect(buffer: &mut [u32], x: usize, y: usize, width_px: usize, height_px: usize,
             buffer_width: usize, buffer_height: usize, color: u32) {
    for py in y..std::cmp::min(y + height_px, buffer_height) {
        for px in x..std::cmp::min(x + width_px, buffer_width) {
            let idx = py * buffer_width + px;
            if idx < buffer.len() {
                buffer[idx] = color;
            }
        }
    }
}

/// Draw text using the bitmap font
pub fn draw_text_bitmap(buffer: &mut [u32], text: &str, x: usize, y: usize, width: usize, color: u32) {
    let mut cursor_x = x;
    
    for c in text.chars() {
        // Check if the character is in the range of our font
        if c >= ' ' && c <= '~' {
            let char_index = (c as usize) - 32; // 32 is the ASCII code for space
            
            if char_index < FONT_BITMAP.len() {
                let bitmap = FONT_BITMAP[char_index];
                
                // Draw the character
                for row in 0..7 {
                    for col in 0..5 {
                        // Check if this pixel is set in the bitmap
                        if (bitmap[row] & (0b10000000 >> col)) != 0 {
                            let pixel_x = cursor_x + col;
                            let pixel_y = y + row;
                            
                            // Check if the pixel is within bounds
                            if pixel_x < width {
                                let idx = pixel_y * width + pixel_x;
                                if idx < buffer.len() {
                                    buffer[idx] = color;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Move cursor to next position (6 pixels per character)
        cursor_x += 6;
        
        // Check if we've reached the end of the display area
        if cursor_x >= width - 6 {
            break;
        }
    }
}