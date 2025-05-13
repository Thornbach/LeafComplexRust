// src/gui/mod.rs - Main GUI module definition

mod state;
mod render;
mod events;
mod components;
mod font; // Added font module

use std::path::PathBuf;
use std::time::Duration;
use minifb::{Window, WindowOptions};

use crate::config::Config;
use crate::errors::Result;
use crate::image_io::load_image;

use self::state::GuiState;

/// Run the GUI application
pub fn run_gui(image_path: PathBuf, config: Config) -> Result<()> {
    println!("Starting GUI with image: {}", image_path.display());
    
    // Load the input image
    let input_image = load_image(&image_path)?;
    println!("Image loaded: {}x{}", input_image.image.width(), input_image.image.height());
    
    // Create resized image for GUI mode if configured
    let processed_image = if let Some(dimensions) = config.gui_resize_dimensions {
        println!("Resizing image for GUI mode to {}x{}", dimensions[0], dimensions[1]);
        crate::image_utils::resize_image(&input_image.image, dimensions)
    } else if let Some(dimensions) = config.resize_dimensions {
        println!("Resizing image to {}x{}", dimensions[0], dimensions[1]);
        crate::image_utils::resize_image(&input_image.image, dimensions)
    } else {
        input_image.image.clone()
    };
    
    // Create window
    let mut window = Window::new(
        "LeafComplexR Visualizer",
        state::WINDOW_WIDTH,
        state::WINDOW_HEIGHT,
        WindowOptions {
            resize: false,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| crate::errors::LeafComplexError::Other(format!("Failed to create window: {}", e)))?;
    
    // Set window limits
    window.limit_update_rate(Some(Duration::from_millis(50))); // 20 FPS
    
    // Create GUI state
    let mut state = GuiState::new(processed_image, config);
    
    // Run initial analysis
    state.update_analysis()?;
    
    // Main loop
    println!("Entering main loop");
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        // Handle events (mouse and keyboard)
        events::handle_events(&mut window, &mut state)?;
        
        // Update the buffer for rendering
        render::update_buffer(&mut state);
        
        // Update the window
        window
            .update_with_buffer(&state.buffer, state::WINDOW_WIDTH, state::WINDOW_HEIGHT)
            .map_err(|e| crate::errors::LeafComplexError::Other(format!("Failed to update window: {}", e)))?;
    }
    
    println!("GUI closed normally");
    Ok(())
}

// Re-export font for potential use outside the GUI module
pub use self::font::FONT_BITMAP;