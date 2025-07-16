// Updated Config struct in src/config.rs with adaptive opening parameters

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
    
    // NEW: Adaptive Opening Parameters
    #[serde(default = "default_adaptive_opening_max_density")]
    pub adaptive_opening_max_density: f64,
    
    #[serde(default = "default_adaptive_opening_max_percentage")]
    pub adaptive_opening_max_percentage: f64,
    
    #[serde(default = "default_adaptive_opening_min_percentage")]
    pub adaptive_opening_min_percentage: f64,
    
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
    
    // Dynamic Thornfiddle Lobe Detection Parameters
    #[serde(default = "default_thornfiddle_max_opening_percentage")]
    pub thornfiddle_max_opening_percentage: f64,
    
    #[serde(default = "default_thornfiddle_min_opening_percentage")]
    pub thornfiddle_min_opening_percentage: f64,
    
    // DEPRECATED: Keep for backward compatibility but not used
    #[serde(default = "default_thornfiddle_opening_size_percentage")]
    pub thornfiddle_opening_size_percentage: f64,
    
    #[serde(default = "default_thornfiddle_pixel_threshold")]
    pub thornfiddle_pixel_threshold: u32,
    
    #[serde(default = "default_thornfiddle_marked_color_rgb")]
    pub thornfiddle_marked_color_rgb: [u8; 3],
    
    // Harmonic Enhancement Control Parameters
    #[serde(default = "default_harmonic_strength_multiplier")]
    pub harmonic_strength_multiplier: f64,
    
    #[serde(default = "default_harmonic_min_chain_length")]
    pub harmonic_min_chain_length: usize,
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

// NEW: Adaptive Opening default functions
fn default_adaptive_opening_max_density() -> f64 {
    25.0 // 25% non-transparent pixel threshold
}

fn default_adaptive_opening_max_percentage() -> f64 {
    10.0 // 10% opening kernel size at maximum density
}

fn default_adaptive_opening_min_percentage() -> f64 {
    1.0 // 1% minimum opening kernel size
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

// Dynamic Thornfiddle default functions
fn default_thornfiddle_max_opening_percentage() -> f64 {
    30.0 // Maximum opening percentage for circular leaves (shape_index = 1.0)
}

fn default_thornfiddle_min_opening_percentage() -> f64 {
    10.0 // Minimum opening percentage for very elongated leaves
}

// DEPRECATED: Legacy parameter for backward compatibility
fn default_thornfiddle_opening_size_percentage() -> f64 {
    20.0 // Legacy default - not used in dynamic calculation
}

fn default_thornfiddle_pixel_threshold() -> u32 {
    3 // Minimum golden pixels to trigger harmonic chain
}

fn default_thornfiddle_marked_color_rgb() -> [u8; 3] {
    [255, 215, 0] // Golden yellow for lobe regions
}

// Harmonic default functions
fn default_harmonic_strength_multiplier() -> f64 {
    1.0 // Default harmonic strength (1.0 = normal, 2.0 = double strength, etc.)
}

fn default_harmonic_min_chain_length() -> usize {
    10 // Minimum chain length to be counted as a harmonic chain
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
            adaptive_opening_max_density: 25.0,
            adaptive_opening_max_percentage: 10.0,
            adaptive_opening_min_percentage: 1.0,
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
            thornfiddle_max_opening_percentage: 30.0,
            thornfiddle_min_opening_percentage: 10.0,
            thornfiddle_opening_size_percentage: 20.0, // Legacy - not used
            thornfiddle_pixel_threshold: 3,
            thornfiddle_marked_color_rgb: [255, 215, 0],
            harmonic_strength_multiplier: 1.0,
            harmonic_min_chain_length: 10,
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
        
        // NEW: Validate adaptive opening parameters
        if self.adaptive_opening_max_density <= 0.0 || self.adaptive_opening_max_density > 100.0 {
            return Err(LeafComplexError::Config(
                "adaptive_opening_max_density must be between 0.0 and 100.0".to_string(),
            ));
        }
        
        if self.adaptive_opening_max_percentage <= 0.0 || self.adaptive_opening_max_percentage > 50.0 {
            return Err(LeafComplexError::Config(
                "adaptive_opening_max_percentage must be between 0.0 and 50.0".to_string(),
            ));
        }
        
        if self.adaptive_opening_min_percentage <= 0.0 || self.adaptive_opening_min_percentage >= self.adaptive_opening_max_percentage {
            return Err(LeafComplexError::Config(
                "adaptive_opening_min_percentage must be > 0.0 and < adaptive_opening_max_percentage".to_string(),
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
        
        // Validate dynamic thornfiddle parameters
        if self.thornfiddle_max_opening_percentage <= 0.0 || self.thornfiddle_max_opening_percentage > 50.0 {
            return Err(LeafComplexError::Config(
                "thornfiddle_max_opening_percentage must be between 0.0 and 50.0".to_string(),
            ));
        }
        
        if self.thornfiddle_min_opening_percentage <= 0.0 || self.thornfiddle_min_opening_percentage >= self.thornfiddle_max_opening_percentage {
            return Err(LeafComplexError::Config(
                "thornfiddle_min_opening_percentage must be > 0.0 and < thornfiddle_max_opening_percentage".to_string(),
            ));
        }
        
        if self.thornfiddle_pixel_threshold == 0 {
            return Err(LeafComplexError::Config(
                "thornfiddle_pixel_threshold must be > 0".to_string(),
            ));
        }
        
        // Validate harmonic parameters
        if self.harmonic_strength_multiplier <= 0.0 {
            return Err(LeafComplexError::Config(
                "harmonic_strength_multiplier must be > 0.0".to_string(),
            ));
        }
        
        if self.harmonic_min_chain_length == 0 {
            return Err(LeafComplexError::Config(
                "harmonic_min_chain_length must be > 0".to_string(),
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