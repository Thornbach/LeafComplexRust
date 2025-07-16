// Enhanced src/lib.rs - Updated with simplified Thornfiddle functions

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

// Re-export Thornfiddle analysis functions (simplified)
pub use thornfiddle::{
    calculate_spectral_entropy_from_contour,
    calculate_spectral_entropy_from_pink_path,
    calculate_approximate_entropy_from_pink_path,
    calculate_edge_feature_density,
    // Golden Pixel Harmonic Thornfiddle functions (with weighted chain scoring)
    calculate_thornfiddle_path_harmonic,           // Main harmonic function with chain intensity weighting
    calculate_leaf_circumference,
    extract_harmonic_thornfiddle_path_signal,
    calculate_spectral_entropy_from_harmonic_thornfiddle_path, // Simplified: removed rhythm analysis
    create_thornfiddle_summary,                    // Summary with weighted chain metrics
    // HarmonicResult struct with weighted metrics
    HarmonicResult,
};

// Re-export morphology functions including Thornfiddle image creation
pub use morphology::{
    trace_contour,
    apply_opening,
    calculate_center_of_mass,
    // Golden lobe detection
    create_thornfiddle_image,
};

pub use point_analysis::{
    calculate_emerge_point,
    get_reference_point,
};