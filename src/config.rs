use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, ErrorKind}; // Added missing import
use std::path::{Path, PathBuf};

use crate::errors::{LeafComplexError, Result};

/// Configuration for LeafComplexR
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    // Existing fields...
    pub input_path: String,
    pub output_base_dir: String,
    pub resize_dimensions: Option<[u32; 2]>,
    
    // Add a specific resize option for GUI mode
    #[serde(default = "default_gui_resize")]
    pub gui_resize_dimensions: Option<[u32; 2]>,
    
    // Other fields...
    pub opening_kernel_size: u32,
    pub marked_region_color_rgb: [u8; 3],
    pub reference_point_choice: ReferencePointChoice,
    pub golden_spiral_rotation_steps: u32,
    pub golden_spiral_phi_exponent_factor: f64,
    #[serde(default = "default_parallel")]
    pub use_parallel: bool,
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

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            // Fixed this to avoid custom() which doesn't exist
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
            gui_resize_dimensions: Some([512, 512]), // Default GUI resize to 512x512
            opening_kernel_size: 5,
            marked_region_color_rgb: [255, 0, 255], // Bright pink
            reference_point_choice: ReferencePointChoice::Com,
            golden_spiral_rotation_steps: 36,
            golden_spiral_phi_exponent_factor,
            use_parallel: true,
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

        // Create output directories if they don't exist
        let base_dir = PathBuf::from(&self.output_base_dir);
        let lec_dir = base_dir.join("LEC");
        let lmc_dir = base_dir.join("LMC");

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