// Updated Config struct in src/config.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::errors::{LeafComplexError, Result};

/// Configuration for LeafComplexR
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {

    pub input_path: String,
    pub output_base_dir: String,
    pub resize_dimensions: Option<[u32; 2]>,
    
    // Add a specific resize option for GUI mode
    #[serde(default = "default_gui_resize")]
    pub gui_resize_dimensions: Option<[u32; 2]>,
    

    pub opening_kernel_size: u32,
    pub marked_region_color_rgb: [u8; 3],
    pub reference_point_choice: ReferencePointChoice,
    pub golden_spiral_rotation_steps: u32,
    pub golden_spiral_phi_exponent_factor: f64,
    #[serde(default = "default_parallel")]
    pub use_parallel: bool,
    
    // Petiole filtering parameters
    #[serde(default = "default_enable_petiole_filter_lec")]
    pub enable_petiole_filter_lec: bool,
    
    #[serde(default = "default_enable_petiole_filter_edge_complexity")]
    pub enable_petiole_filter_edge_complexity: bool,
    
    #[serde(default = "default_petiole_remove_completely")]
    pub petiole_remove_completely: bool,
    
    // Pink threshold filtering parameters
    #[serde(default = "default_enable_pink_threshold_filter")]
    pub enable_pink_threshold_filter: bool,
    
    #[serde(default = "default_pink_threshold_value")]
    pub pink_threshold_value: f64,
    
    #[serde(default = "default_thornfiddle_smoothing_strength")]
    pub thornfiddle_smoothing_strength: f64,
    
    // New parameter for spectral analysis
    #[serde(default = "default_thornfiddle_interpolation_points")]
    pub thornfiddle_interpolation_points: usize,

    // Approximate Entropy parameters
    #[serde(default = "default_approximate_entropy_m")]
    pub approximate_entropy_m: usize,
    
    #[serde(default = "default_approximate_entropy_r")]
    pub approximate_entropy_r: f64,

    // LEC scaling factor for edge complexity calculation
    #[serde(default = "default_lec_scaling_factor")]
    pub lec_scaling_factor: f64,
    
    // NEW: Thornfiddle Lobe Detection Parameters
    #[serde(default = "default_thornfiddle_opening_size_percentage")]
    pub thornfiddle_opening_size_percentage: f64,
    
    #[serde(default = "default_thornfiddle_pixel_threshold")]
    pub thornfiddle_pixel_threshold: u32,
    
    #[serde(default = "default_thornfiddle_marked_color_rgb")]
    pub thornfiddle_marked_color_rgb: [u8; 3],
}

/// Reference point choice enum
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReferencePointChoice {
    /// EmergePoint
    Ep,
    /// Center of Mass
    Com,
}

fn default_parallel() -> bool {
    true
}

fn default_gui_resize() -> Option<[u32; 2]> {
    Some([512, 512])
}

fn default_enable_petiole_filter_lec() -> bool {
    true
}

fn default_enable_petiole_filter_edge_complexity() -> bool {
    true
}

fn default_petiole_remove_completely() -> bool {
    false // Default to zeroing mode
}

fn default_enable_pink_threshold_filter() -> bool {
    true
}

fn default_pink_threshold_value() -> f64 {
    3.0
}

fn default_thornfiddle_smoothing_strength() -> f64 {
    1.0 // Range: 0.0 (no smoothing) to 10.0 (strong smoothing)
}

// New default function for interpolation points
fn default_thornfiddle_interpolation_points() -> usize {
    1000 // Default to 1000 points for consistent analysis
}

fn default_approximate_entropy_m() -> usize {
    2 // Default pattern length
}

fn default_approximate_entropy_r() -> f64 {
    0.2 // Default tolerance (20% of std deviation)
}

fn default_lec_scaling_factor() -> f64 {
    3.0 // Default scaling factor for edge complexity
}

// NEW: Thornfiddle default functions
fn default_thornfiddle_opening_size_percentage() -> f64 {
    10.0 // 10% of image width for aggressive opening
}

fn default_thornfiddle_pixel_threshold() -> u32 {
    3 // Minimum golden pixels to trigger harmonic chain
}

fn default_thornfiddle_marked_color_rgb() -> [u8; 3] {
    [255, 215, 0] // Golden yellow for lobe regions
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            LeafComplexError::Config(format!("Failed to read config file '{}': {}", path.display(), e))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            LeafComplexError::Config(format!("Failed to parse config file '{}': {}", path.display(), e))
        })?;

        Ok(config)
    }

    /// Create default configuration
    pub fn default() -> Self {
        let golden_spiral_phi_exponent_factor = 2.0 / std::f64::consts::PI;

        Self {
            input_path: "./input".to_string(),
            output_base_dir: "./output".to_string(),
            resize_dimensions: Some([800, 600]),
            gui_resize_dimensions: Some([512, 512]),
            opening_kernel_size: 5,
            marked_region_color_rgb: [255, 0, 255], // Bright pink
            reference_point_choice: ReferencePointChoice::Com,
            golden_spiral_rotation_steps: 36,
            golden_spiral_phi_exponent_factor,
            use_parallel: true,
            enable_petiole_filter_lec: true,
            enable_petiole_filter_edge_complexity: true,
            petiole_remove_completely: false,
            enable_pink_threshold_filter: true,
            pink_threshold_value: 3.0,
            thornfiddle_smoothing_strength: 1.0,
            thornfiddle_interpolation_points: 1000,
            approximate_entropy_m: 2,
            approximate_entropy_r: 0.2,
            lec_scaling_factor: 3.0,
            thornfiddle_opening_size_percentage: 10.0,
            thornfiddle_pixel_threshold: 3,
            thornfiddle_marked_color_rgb: [255, 215, 0],
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check input path exists
        let input_path = PathBuf::from(&self.input_path);
        if !input_path.exists() {
            return Err(LeafComplexError::InvalidPath(input_path));
        }

        // Check kernel size is reasonable
        if self.opening_kernel_size == 0 {
            return Err(LeafComplexError::Config(
                "opening_kernel_size must be > 0".to_string(),
            ));
        }

        // Validate golden spiral parameters
        if self.golden_spiral_phi_exponent_factor <= 0.0 {
            return Err(LeafComplexError::Config(
                "golden_spiral_phi_exponent_factor must be > 0.0".to_string(),
            ));
        }
        
        // Validate thornfiddle parameters
        if self.thornfiddle_interpolation_points < 10 {
            return Err(LeafComplexError::Config(
                "thornfiddle_interpolation_points must be >= 10".to_string(),
            ));
        }

        // Validate approximate entropy parameters
        if self.approximate_entropy_m < 1 {
            return Err(LeafComplexError::Config(
                "approximate_entropy_m must be >= 1".to_string(),
            ));
        }

        if self.approximate_entropy_r <= 0.0 {
            return Err(LeafComplexError::Config(
                "approximate_entropy_r must be > 0.0".to_string(),
            ));
        }
        
        // NEW: Validate thornfiddle parameters
        if self.thornfiddle_opening_size_percentage <= 0.0 || self.thornfiddle_opening_size_percentage > 50.0 {
            return Err(LeafComplexError::Config(
                "thornfiddle_opening_size_percentage must be between 0.0 and 50.0".to_string(),
            ));
        }
        
        if self.thornfiddle_pixel_threshold == 0 {
            return Err(LeafComplexError::Config(
                "thornfiddle_pixel_threshold must be > 0".to_string(),
            ));
        }

        // Create output directories if they don't exist
        let base_dir = PathBuf::from(&self.output_base_dir);
        let lec_dir = base_dir.join("LEC");
        let lmc_dir = base_dir.join("LMC");
        let thornfiddle_dir = base_dir.join("Thornfiddle");

        fs::create_dir_all(&lec_dir).map_err(|e| {
            LeafComplexError::Io(io::Error::new(
                ErrorKind::Other,
                format!("Failed to create LEC output directory: {}", e),
            ))
        })?;

        fs::create_dir_all(&lmc_dir).map_err(|e| {
            LeafComplexError::Io(io::Error::new(
                ErrorKind::Other,
                format!("Failed to create LMC output directory: {}", e),
            ))
        })?;
        
        fs::create_dir_all(&thornfiddle_dir).map_err(|e| {
            LeafComplexError::Io(io::Error::new(
                ErrorKind::Other,
                format!("Failed to create Thornfiddle output directory: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            LeafComplexError::Config(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(path, content).map_err(|e| LeafComplexError::Io(e))?;

        Ok(())
    }
}