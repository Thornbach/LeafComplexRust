// src/gui/render.rs - Rendering functions

use super::state::{
    GuiState, WINDOW_WIDTH, WINDOW_HEIGHT, 
    COLOR_BACKGROUND, COLOR_TEXT, COLOR_SLIDER_BG, COLOR_SLIDER_FG, COLOR_SLIDER_HOVER,
    COLOR_REFERENCE_POINT, COLOR_CONTOUR_POINT, COLOR_SELECTED_POINT,
    COLOR_STRAIGHT_PATH, COLOR_GOLDEN_PATH, COLOR_RIGHT_SPIRAL_PATH, COLOR_DIEGO_PATH,
    COLOR_CLR_ALPHA, COLOR_CLR_GAMMA
};
use super::components::{draw_circle, draw_rect, draw_text_bitmap};

/// Update the buffer for display
pub fn update_buffer(state: &mut GuiState) {

    for pixel in &mut state.buffer {
        *pixel = COLOR_BACKGROUND;
    }
    
    // Draw image
    if let Some(img) = &state.marked_image {
        let img_width = img.width() as usize;
        let img_height = img.height() as usize;
        
        // Draw the image
        for y in 0..img_height {
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            if display_y >= WINDOW_HEIGHT {
                continue;
            }
            
            for x in 0..img_width {
                let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                if display_x >= state.display_width {
                    continue;
                }
                
                let pixel = img.get_pixel(x as u32, y as u32);
                
                // Handle transparency visualization
                if state.show_transparency && pixel[3] == 0 {
                    // Show transparent pixels as a checkerboard pattern
                    let checker = (x + y) % 2 == 0;
                    let color = if checker { 0x606060 } else { 0x404040 };
                    
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < state.buffer.len() {
                        state.buffer[idx] = color;
                    }
                }
                // Skip fully transparent pixels unless showing transparency
                else if pixel[3] > 0 {
                    let r = pixel[0] as u32;
                    let g = pixel[1] as u32;
                    let b = pixel[2] as u32;
                    let color = (r << 16) | (g << 8) | b;
                    
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < state.buffer.len() {
                        state.buffer[idx] = color;
                    }
                }
            }
        }
        
        // Draw CLR regions if enabled
        if state.show_clr_regions {
            // Draw CLR_Alpha (transparent) pixels
            for (i, &(x, y)) in state.clr_alpha_pixels.iter().enumerate() {
                // Sample every few pixels for performance
                if i % 4 != 0 {
                    continue;
                }
                
                let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
                
                if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < state.buffer.len() {
                        state.buffer[idx] = COLOR_CLR_ALPHA;
                    }
                }
            }
            
            // Draw CLR_Gamma (non-transparent) pixels
            for (i, &(x, y)) in state.clr_gamma_pixels.iter().enumerate() {
                // Sample every few pixels for performance
                if i % 4 != 0 {
                    continue;
                }
                
                let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
                
                if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < state.buffer.len() {
                        state.buffer[idx] = COLOR_CLR_GAMMA;
                    }
                }
            }

            if state.show_right_spiral && !state.right_clr_alpha_pixels.is_empty() {
                // Draw right spiral CLR regions with different colors
                for (i, &(x, y)) in state.right_clr_alpha_pixels.iter().enumerate() {
                    // Sample every few pixels for performance
                    if i % 4 != 0 {
                        continue;
                    }
                    
                    let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                    let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
                    
                    if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                        let idx = display_y * WINDOW_WIDTH + display_x;
                        if idx < state.buffer.len() {
                            // Use a slightly different color for right spiral
                            state.buffer[idx] = 0xFF0000A0; // More reddish
                        }
                    }
                }

                if !state.right_clr_gamma_pixels.is_empty() {
                    // Draw right spiral CLR gamma pixels
                    for (i, &(x, y)) in state.right_clr_gamma_pixels.iter().enumerate() {
                        // Sample every few pixels for performance
                        if i % 4 != 0 {
                            continue;
                        }
                        
                        let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                        let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
                        
                        if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                            let idx = display_y * WINDOW_WIDTH + display_x;
                            if idx < state.buffer.len() {
                                // Use a slightly different color for right spiral
                                state.buffer[idx] = 0xFF8000A0; // More orangeish
                            }
                        }
                    }
                }
            }
        }
        
        // Draw paths
        
        // Draw straight path
        for &(x, y) in &state.straight_path {
            let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            
            if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                let idx = display_y * WINDOW_WIDTH + display_x;
                if idx < state.buffer.len() {
                    state.buffer[idx] = COLOR_STRAIGHT_PATH;
                }
            }
        }
        
        // Draw DiegoPath (always draw if available)
        for &(x, y) in &state.diego_path {
            let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            
            if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                let idx = display_y * WINDOW_WIDTH + display_x;
                if idx < state.buffer.len() {
                    state.buffer[idx] = COLOR_DIEGO_PATH;
                }
            }
        }
        
        // Draw golden path
        for &(x, y) in &state.left_spiral_path {
            let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            
            if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                let idx = display_y * WINDOW_WIDTH + display_x;
                if idx < state.buffer.len() {
                    state.buffer[idx] = COLOR_GOLDEN_PATH; // Keep the original color for backward compatibility
                }
            }
        }

        // Draw right spiral path if enabled
        if state.show_right_spiral {
            for &(x, y) in &state.right_spiral_path {
                let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
                let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
                
                if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                    let idx = display_y * WINDOW_WIDTH + display_x;
                    if idx < state.buffer.len() {
                        state.buffer[idx] = COLOR_RIGHT_SPIRAL_PATH;
                    }
                }
            }
        }
        
        // Draw contour points
        for (i, &point) in state.lec_contour.iter().enumerate() {
            let display_x = (point.0 as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (point.1 as f32 * state.scale_factor) as usize + state.offset_y;
            
            if display_x < state.display_width && display_y < WINDOW_HEIGHT {
                draw_circle(&mut state.buffer, display_x, display_y, 1, 
                    WINDOW_WIDTH, WINDOW_HEIGHT,
                    if Some(i) == state.selected_point_idx {
                        COLOR_SELECTED_POINT
                    } else {
                        COLOR_CONTOUR_POINT
                    });
            }
        }
        
        // Draw selected point
        if let Some(idx) = state.selected_point_idx {
            let (x, y) = state.lec_contour[idx];
            let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            
            draw_circle(&mut state.buffer, display_x, display_y, 4, 
                WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_SELECTED_POINT);
        }
        
        // Draw reference point
        if let Some((x, y)) = state.reference_point {
            let display_x = (x as f32 * state.scale_factor) as usize + state.offset_x;
            let display_y = (y as f32 * state.scale_factor) as usize + state.offset_y;
            
            draw_circle(&mut state.buffer, display_x, display_y, 5, 
                WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_REFERENCE_POINT);
        }
    }
    
    // Draw info panel
    draw_info_panel(state);
}

/// Draw the info panel
fn draw_info_panel(state: &mut GuiState) {
    // Draw info panel background
    let info_panel_x = state.display_width;
    for y in 0..WINDOW_HEIGHT {
        for x in info_panel_x..WINDOW_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            state.buffer[idx] = 0x202020; // Dark gray
        }
    }
    
    // Draw info panel separator
    for y in 0..WINDOW_HEIGHT {
        let idx = y * WINDOW_WIDTH + info_panel_x;
        state.buffer[idx] = 0x505050; // Medium gray
    }
    
    // Draw text in info panel
    let panel_x = state.display_width + 10;
    let mut text_y = 20;
    
    // Title
    draw_text_bitmap(&mut state.buffer, "LeafComplexR Visualizer", panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
    text_y += 30;
    
    // Kernel size
    draw_text_bitmap(&mut state.buffer, &format!("Kernel Size: {}", state.kernel_size), 
            panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
    text_y += 20;
    
    // Store the Y coordinate for the slider and draw it
    state.slider_y_coord = text_y;

    // Draw a slider for kernel size
    let slider_x = panel_x;
    let slider_width = super::state::INFO_PANEL_WIDTH - 20;
    let slider_handle_pos = state.get_slider_position();
    
    // Slider track
    for x_pos in slider_x..(slider_x + slider_width) {
        let idx = state.slider_y_coord * WINDOW_WIDTH + x_pos;
        if idx < state.buffer.len() {
            state.buffer[idx] = COLOR_SLIDER_BG;
        }
    }
    
    // Slider handle
    let handle_color = if state.is_mouse_on_slider() || state.slider_dragging {
        COLOR_SLIDER_HOVER
    } else {
        COLOR_SLIDER_FG
    };
    
    draw_circle(&mut state.buffer, slider_handle_pos, state.slider_y_coord, 5, 
               WINDOW_WIDTH, WINDOW_HEIGHT, handle_color);
    
    text_y += 30;
    
    // Reference point
    if let Some(point) = state.reference_point {
        draw_text_bitmap(&mut state.buffer, &format!("Reference Point: ({}, {})", point.0, point.1), 
                panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
    }
    text_y += 20;
    
    // Transparency check
    if state.selected_point_idx.is_some() {
        let result_str = if state.transparency_check_result { "YES" } else { "NO" };
        draw_text_bitmap(&mut state.buffer, &format!("Crosses transparency: {}", result_str), 
                 panel_x, text_y, WINDOW_WIDTH, COLOR_TEXT);
        text_y += 20;
    }
    
    // Selected point info
    draw_selected_point_info(state, panel_x, &mut text_y);
    
    // Draw legend
    draw_legend(state, panel_x, &mut text_y);
    
    // Controls
    draw_controls(state, panel_x, &mut text_y);
    
    // Status message at bottom
    let status_y = WINDOW_HEIGHT - 20;
    draw_text_bitmap(&mut state.buffer, &state.status_message, panel_x, status_y, WINDOW_WIDTH, COLOR_TEXT);
}

/// Draw information about the selected point
fn draw_selected_point_info(state: &mut GuiState, panel_x: usize, text_y: &mut usize) {
    if let (Some(idx), Some(features)) = (state.selected_point_idx, &state.selected_features) {
        let point = state.lec_contour[idx];
        
        *text_y += 10;
        draw_text_bitmap(&mut state.buffer, "Selected Point Data:", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        draw_text_bitmap(&mut state.buffer, &format!("Point ({}, {}) [idx: {}]", point.0, point.1, idx), 
                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        draw_text_bitmap(&mut state.buffer, &format!("StraightPath: {:.2}", features.straight_path_length), 
                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        // Always show DiegoPath info
        draw_text_bitmap(&mut state.buffer, &format!("DiegoPath: {:.2}", features.diego_path_length), 
                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        draw_text_bitmap(&mut state.buffer, &format!("DiegoPath %: {:.2}", features.diego_path_perc), 
                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        if let Some(diego_pink) = features.diego_path_pink {
            draw_text_bitmap(&mut state.buffer, &format!("DiegoPath_Pink: {}", diego_pink), 
                    panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
        }
        
        if features.gyro_path_length > 0.0 {
            draw_text_bitmap(&mut state.buffer, &format!("GyroPath: {:.2}", features.gyro_path_length), 
                    panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            draw_text_bitmap(&mut state.buffer, &format!("GyroPath %: {:.2}", features.gyro_path_perc), 
                    panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            // Display averaged CLR values
            draw_text_bitmap(&mut state.buffer, &format!("Avg CLR_Alpha: {}", features.clr_alpha), 
                    panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            draw_text_bitmap(&mut state.buffer, &format!("Avg CLR_Gamma: {}", features.clr_gamma), 
                    panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            // Display individual CLR values if right spiral is shown
            draw_text_bitmap(&mut state.buffer, &format!("Left CLR_Alpha: {}", features.left_clr_alpha), 
                            panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            draw_text_bitmap(&mut state.buffer, &format!("Left CLR_Gamma: {}", features.left_clr_gamma), 
                            panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
            *text_y += 20;
            
            if state.show_right_spiral {
                draw_text_bitmap(&mut state.buffer, &format!("Right CLR_Alpha: {}", features.right_clr_alpha), 
                                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
                *text_y += 20;
                
                draw_text_bitmap(&mut state.buffer, &format!("Right CLR_Gamma: {}", features.right_clr_gamma), 
                                panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
                *text_y += 20;
            }
        }
    }
}

/// Draw the legend with color explanation
fn draw_legend(state: &mut GuiState, panel_x: usize, text_y: &mut usize) {
    *text_y += 20;
    draw_text_bitmap(&mut state.buffer, "Legend:", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Use color boxes with text
    let color_box_size = 10;
    let color_text_gap = 5;
    
    // Reference Point
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_REFERENCE_POINT);
    draw_text_bitmap(&mut state.buffer, "Reference Point", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Contour Points
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CONTOUR_POINT);
    draw_text_bitmap(&mut state.buffer, "Contour Points", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Selected Point
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_SELECTED_POINT);
    draw_text_bitmap(&mut state.buffer, "Selected Point", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Straight Path
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_STRAIGHT_PATH);
    draw_text_bitmap(&mut state.buffer, "Straight Path", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Diego Path
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_DIEGO_PATH);
    draw_text_bitmap(&mut state.buffer, "Diego Path (Within Leaf)", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Golden Path
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_GOLDEN_PATH);
    draw_text_bitmap(&mut state.buffer, "Left Golden Spiral Path", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    // Right Spiral Path
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
        WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_RIGHT_SPIRAL_PATH);
    draw_text_bitmap(&mut state.buffer, "Right Spiral Path", 
    panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // CLR Alpha
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CLR_ALPHA);
    draw_text_bitmap(&mut state.buffer, "CLR_Alpha (Transparent)", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // CLR Gamma
    draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
             WINDOW_WIDTH, WINDOW_HEIGHT, COLOR_CLR_GAMMA);
    draw_text_bitmap(&mut state.buffer, "CLR_Gamma (Non-Transparent)", 
            panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;
    
    // Right CLR Alpha (only show if right spiral is toggled)
    if state.show_right_spiral {
        draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
                WINDOW_WIDTH, WINDOW_HEIGHT, 0xFF0000A0);
        draw_text_bitmap(&mut state.buffer, "Right CLR_Alpha", 
                panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
        
        // Right CLR Gamma
        draw_rect(&mut state.buffer, panel_x, *text_y, color_box_size, color_box_size, 
                WINDOW_WIDTH, WINDOW_HEIGHT, 0xFF8000A0);
        draw_text_bitmap(&mut state.buffer, "Right CLR_Gamma", 
                panel_x + color_box_size + color_text_gap, *text_y, WINDOW_WIDTH, COLOR_TEXT);
        *text_y += 20;
    }
}

/// Draw the controls section
fn draw_controls(state: &mut GuiState, panel_x: usize, text_y: &mut usize) {
    *text_y += 5;
    
    draw_text_bitmap(&mut state.buffer, "Controls:", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- Click: Select contour point", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- H/L: Previous/Next point", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- R: Toggle right spiral path", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- T: Toggle transparency view", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- C: Toggle CLR regions", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
    *text_y += 20;

    draw_text_bitmap(&mut state.buffer, "- Esc: Exit", panel_x, *text_y, WINDOW_WIDTH, COLOR_TEXT);
}

