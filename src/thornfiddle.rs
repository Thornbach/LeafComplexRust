// src/thornfiddle.rs - Thornfiddle complexity analysis for leaf morphology

use std::path::Path;
use std::fs;
use rustfft::{FftPlanner, num_complex::Complex};
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;
use crate::image_utils::is_non_transparent;

/// Calculate the Thornfiddle Multiplier for a given feature point
pub fn calculate_thornfiddle_multiplier(feature: &MarginalPointFeatures) -> f64 {
    // Skip calculation if DiegoPath or StraightPath is invalid
    if feature.straight_path_length <= 0.0 || feature.diego_path_length <= 0.0 {
        return 1.0;
    }
    
    // Calculate Path_Complexity using hyperbolic tangent
    let path_ratio = feature.diego_path_length / feature.straight_path_length;
    let path_complexity = ((path_ratio - 1.0) * 3.0).tanh();
    
    // Calculate Region_Factor with square root
    let clr_sum = (feature.clr_alpha + feature.clr_gamma) as f64;
    let denominator = feature.straight_path_length.powi(2) + 1.0;
    let region_factor = (clr_sum / denominator).sqrt();
    
    // Calculate final multiplier
    1.0 + path_complexity * (1.0 + region_factor)
}

/// Calculate Thornfiddle Path for a feature (DiegoPath * Multiplier)
pub fn calculate_thornfiddle_path(feature: &MarginalPointFeatures) -> f64 {
    let multiplier = calculate_thornfiddle_multiplier(feature);
    feature.diego_path_length * multiplier
}

pub fn calculate_spectral_entropy(thornfiddle_paths: &[f64]) -> f64 {
    if thornfiddle_paths.len() < 4 {
        // Not enough data points for meaningful spectral analysis
        return 0.0;
    }

    let original_len = thornfiddle_paths.len();

    // 1. Detrend (remove mean) - Optional, but good practice
    let mean: f64 = thornfiddle_paths.iter().sum::<f64>() / original_len as f64;
    let mut processed_series: Vec<f64> = thornfiddle_paths.iter().map(|&x| x - mean).collect();

    // 2. Apply Hann Window to the actual data
    // Ensure original_len > 1 to avoid division by zero if window is (N-1)
    if original_len > 1 { // Windowing a single point is trivial (value becomes 0 or itself)
        for i in 0..original_len {
            let hann_factor = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (original_len - 1) as f64).cos());
            processed_series[i] *= hann_factor;
        }
    } else if original_len == 1 {
        // For a single point, after mean removal, it's 0. Hann factor would be 0.5 * (1-cos(0)) = 0.
        // So the series effectively becomes [0.0]. This will result in 0 entropy later.
        processed_series[0] = 0.0; // Or handle as a special case returning 0 earlier
    }


    // 3. Pad series to a power of 2 for FFT efficiency
    let mut padded_series = processed_series; // Already a Vec
    // Find next power of 2
    let mut fft_size = 1;
    while fft_size < original_len {
        fft_size *= 2;
    }
    if fft_size < 4 { // Ensure fft_size is at least 4 if original_len was < 4 but passed the initial guard somehow
        fft_size = 4; // Smallest fft_size for power_spectrum to have at least one element
    }

    // Pad with zeros
    padded_series.resize(fft_size, 0.0);

    // 4. Convert to complex numbers for FFT
    let mut complex_input: Vec<Complex<f64>> = padded_series
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();

    // 5. Create FFT planner and forward FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    // Perform FFT
    fft.process(&mut complex_input);

    // 6. Calculate power spectrum (only use first half due to symmetry, excluding DC)
    // We need at least one AC component for meaningful entropy.
    // fft_size/2 gives the number of unique points including DC and Nyquist (if even).
    // We want from index 1 up to, but not including, Nyquist.
    // If fft_size = 4, unique points are 0, 1 (Nyquist). We want only index 1, so `1..fft_size/2` -> `1..2`
    // power_spectrum will have (fft_size / 2) - 1 elements.
    // Smallest fft_size is 4 (due to original_len >= 4 or forced above).
    // So, smallest power_spectrum len is 4/2 - 1 = 1.
    if fft_size < 4 { // Should not happen due to earlier checks/adjustments
        return 0.0; // Or handle error, not enough bins for AC components
    }
    let power_spectrum: Vec<f64> = complex_input[1..fft_size / 2]
        .iter()
        .map(|c| c.norm_sqr())
        .collect();

    if power_spectrum.is_empty() {
        // This can happen if fft_size is 2, then 1..fft_size/2 is 1..1, an empty range.
        // Our fft_size is guaranteed to be at least 4.
        return 0.0;
    }

    // 7. Normalize to create probability distribution
    let total_power: f64 = power_spectrum.iter().sum();
    if total_power <= 1e-10 { // Use a small epsilon for floating point comparison
        return 0.0; // No power in AC components
    }

    let probabilities: Vec<f64> = power_spectrum
        .iter()
        .map(|&p| p / total_power)
        .collect();

    // 8. Calculate Shannon entropy
    let entropy: f64 = -probabilities
        .iter()
        .filter(|&&p| p > 1e-10) // Filter out zero probabilities to avoid log(0)
        .map(|&p| p * p.log2())
        .sum::<f64>();

    // 9. Normalize by maximum possible entropy
    // Max entropy is log2(N) where N is the number of bins in the probability distribution.
    let num_bins = probabilities.len() as f64;
    if num_bins <= 1.0 { // If only one bin, max_entropy is log2(1) = 0. Avoid division by zero.
                         // entropy itself would also be 0 (-1.0 * 1.0.log2() = 0)
        return 0.0; // Or just `entropy` which would be 0
    }

    let max_entropy = num_bins.log2();
    if max_entropy <= 1e-10 { // Effectively if num_bins was 1.
        entropy // Which should be 0
    } else {
        entropy / max_entropy
    }
}

/// Calculate spectral entropy for a set of features
pub fn calculate_features_spectral_entropy(features: &[MarginalPointFeatures]) -> f64 {
    // Extract Thornfiddle path values
    let thornfiddle_paths: Vec<f64> = features
        .iter()
        .map(|feature| calculate_thornfiddle_path(feature))
        .collect();
    
    // Calculate spectral entropy
    calculate_spectral_entropy(&thornfiddle_paths)
}

/// Create Thornfiddle summary CSV with spectral entropy, circularity, and area
pub fn create_thornfiddle_summary<P: AsRef<Path>>(
    output_dir: P,
    filename: &str,
    spectral_entropy: f64,
    circularity: f64,
    area: u32,
) -> Result<()> {
    // Create Thornfiddle directory if it doesn't exist
    let thornfiddle_dir = output_dir.as_ref().join("Thornfiddle");
    fs::create_dir_all(&thornfiddle_dir).map_err(|e| LeafComplexError::Io(e))?;
    
    // Path to summary CSV
    let summary_path = thornfiddle_dir.join("summary.csv");
    
    // Check if summary file already exists
    let file_exists = summary_path.exists();
    
    // Open file in append mode if it exists, otherwise create new
    let mut writer = if file_exists {
        Writer::from_writer(fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&summary_path)
            .map_err(|e| LeafComplexError::Io(e))?)
    } else {
        let mut writer = Writer::from_path(&summary_path)
            .map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        // Write header only for new file
        writer.write_record(&[
            "ID",
            "Spectral_Entropy",
            "Circularity",
            "Area",
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data
    writer.write_record(&[
        filename,
        &format!("{:.6}", spectral_entropy),
        &format!("{:.6}", circularity),
        &area.to_string(),
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}