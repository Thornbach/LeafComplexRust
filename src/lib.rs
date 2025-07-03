// src/lib.rs - Updated to include golden pixel thornfiddle functions

pub mod config;
pub mod errors;
pub mod feature_extraction;
pub mod gui; 
pub mod image_io;
pub mod image_utils;
pub mod morphology;
pub mod path_algorithms;
pub mod pipeline;
pub mod point_analysis;
pub mod output;
pub mod thornfiddle;
pub mod shape_analysis;

// Re-export commonly used types and functions for easier library access
pub use errors::{LeafComplexError, Result};
pub use config::Config;
pub use pipeline::process_image;
pub use image_io::{InputImage, load_image, save_image};

// Re-export the shape analysis functions for direct library usage
pub use shape_analysis::{
    analyze_shape,
    analyze_shape_comprehensive,
    calculate_biological_dimensions,
    calculate_biological_dimensions_fast,
    calculate_bounding_box_dimensions, // Keep for backward compatibility
    calculate_outline_count,
    calculate_outline_count_from_contour,
    calculate_circularity_from_contour,
    calculate_area,
    calculate_circularity,
};

// Re-export other useful analysis functions including NEW golden pixel functions
pub use thornfiddle::{
    calculate_spectral_entropy_from_contour,
    calculate_spectral_entropy_from_pink_path,
    calculate_approximate_entropy_from_pink_path,
    calculate_edge_feature_density,
    // NEW: Golden Pixel Harmonic Thornfiddle functions
    calculate_thornfiddle_path_harmonic,
    calculate_leaf_circumference,
    extract_harmonic_thornfiddle_path_signal,
    calculate_spectral_entropy_from_harmonic_thornfiddle_path,
};

// Re-export morphology functions including NEW Thornfiddle image creation
pub use morphology::{
    trace_contour,
    apply_opening,
    calculate_center_of_mass,
    // NEW: Golden lobe detection
    create_thornfiddle_image,
};

pub use point_analysis::{
    calculate_emerge_point,
    get_reference_point,
};