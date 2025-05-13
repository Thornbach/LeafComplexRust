// src/gui/events.rs - Event handling for mouse and keyboard

use std::time::Instant;
use minifb::{Window, Key, MouseButton, MouseMode};

use crate::errors::Result;
use super::state::GuiState;

/// Handle all events (mouse and keyboard)
pub fn handle_events(window: &mut Window, state: &mut GuiState) -> Result<()> {
    // Get mouse position
    if let Some((x, y)) = window.get_mouse_pos(MouseMode::Discard) {
        state.mouse_x = x as usize;
        state.mouse_y = y as usize;
    }
    
    // Get mouse button state
    let mouse_down_now = window.get_mouse_down(MouseButton::Left);
    
    // Handle mouse click or drag
    if mouse_down_now {
        if !state.mouse_down {
            // Initial click
            if state.mouse_x < state.display_width {
                // Click on image area - select contour point
                if let Some(idx) = state.find_nearest_contour_point(state.mouse_x, state.mouse_y) {
                    if let Err(e) = state.select_point(idx) {
                        state.status_message = format!("Error selecting point: {}", e);
                    }
                }
            } else if state.is_mouse_on_slider() {
                // Click on slider - start dragging
                state.slider_dragging = true;
                if let Err(e) = state.handle_slider_movement() {
                    state.status_message = format!("Error updating kernel size: {}", e);
                }
            }
        } else if state.slider_dragging {
            // Continue dragging slider
            if let Err(e) = state.handle_slider_movement() {
                state.status_message = format!("Error updating kernel size: {}", e);
            }
        }
    } else {
        // Mouse up - stop dragging
        state.slider_dragging = false;
    }
    
    state.mouse_down = mouse_down_now;
    
    // Handle keyboard input with improved key repeat
    let now = Instant::now();
    
    // Handle single key presses without repeat
    if window.is_key_pressed(Key::T, minifb::KeyRepeat::No) {
        state.show_transparency = !state.show_transparency;
        state.status_message = format!("Transparency view: {}", 
                                     if state.show_transparency { "ON" } else { "OFF" });
    }
    
    if window.is_key_pressed(Key::C, minifb::KeyRepeat::No) {
        state.show_clr_regions = !state.show_clr_regions;
        state.status_message = format!("CLR regions view: {}", 
                                     if state.show_clr_regions { "ON" } else { "OFF" });
    }

    if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
        state.show_right_spiral = !state.show_right_spiral;
        state.status_message = format!("Right spiral path: {}", 
                                   if state.show_right_spiral { "ON" } else { "OFF" });
        
        // Update the currently selected point to reflect the change
        if let Some(idx) = state.selected_point_idx {
            if let Err(e) = state.select_point(idx) {
                state.status_message = format!("Error updating selection: {}", e);
            }
        }
    }
    
    // Check if H is pressed (previous point)
    if window.is_key_down(Key::H) {
        let should_process = if let Some(last_key) = state.last_key_pressed {
            if last_key == Key::H {
                // Check if enough time has passed for key repeat
                if let Some(timer) = state.key_repeat_timer {
                    let elapsed = now.duration_since(timer);
                    // Initial delay, then faster repeat
                    let delay = if state.key_repeat_count == 0 {
                        std::time::Duration::from_millis(500) // Initial delay
                    } else {
                        std::time::Duration::from_millis(100) // Repeat delay
                    };
                    
                    elapsed > delay
                } else {
                    true
                }
            } else {
                // New key pressed
                true
            }
        } else {
            // No previous key
            true
        };
        
        if should_process {
            if let Err(e) = state.handle_key_repeat(Key::H, state.selected_point_idx, false) {
                state.status_message = format!("Error selecting point: {}", e);
            }
            
            state.key_repeat_timer = Some(now);
            state.key_repeat_count += 1;
            state.last_key_pressed = Some(Key::H);
        }
    }
    // Check if L is pressed (next point)
    else if window.is_key_down(Key::L) {
        let should_process = if let Some(last_key) = state.last_key_pressed {
            if last_key == Key::L {
                // Check if enough time has passed for key repeat
                if let Some(timer) = state.key_repeat_timer {
                    let elapsed = now.duration_since(timer);
                    // Initial delay, then faster repeat
                    let delay = if state.key_repeat_count == 0 {
                        std::time::Duration::from_millis(500) // Initial delay
                    } else {
                        std::time::Duration::from_millis(100) // Repeat delay
                    };
                    
                    elapsed > delay
                } else {
                    true
                }
            } else {
                // New key pressed
                true
            }
        } else {
            // No previous key
            true
        };
        
        if should_process {
            if let Err(e) = state.handle_key_repeat(Key::L, state.selected_point_idx, true) {
                state.status_message = format!("Error selecting point: {}", e);
            }
            
            state.key_repeat_timer = Some(now);
            state.key_repeat_count += 1;
            state.last_key_pressed = Some(Key::L);
        }
    }
    else {
        // No navigation keys pressed, reset state
        state.key_repeat_timer = None;
        state.key_repeat_count = 0;
        state.last_key_pressed = None;
    }
    
    Ok(())
}