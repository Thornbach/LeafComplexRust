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

/// Apply Z-score normalization to a vector of values
/// Returns the Z-scored values (mean=0, std=1) or zeros if std=0
fn z_score_normalize(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    
    // Calculate mean
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    
    // Calculate standard deviation
    let variance = values.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();
    
    // Apply Z-score normalization
    if std_dev > 1e-10 { // Use small epsilon to avoid division by zero
        values.iter()
            .map(|&x| (x - mean) / std_dev)
            .collect()
    } else {
        // If standard deviation is essentially zero, return zeros
        vec![0.0; values.len()]
    }
}

/// Preprocess thornfiddle paths to handle simple shapes better
pub fn preprocess_thornfiddle_paths(paths: &[f64], circularity: f64) -> Vec<f64> {
    if paths.is_empty() {
        return Vec::new();
    }
    
    // Calculate basic statistics
    let mean = paths.iter().sum::<f64>() / paths.len() as f64;
    let variance = paths.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / paths.len() as f64;
    let std_dev = variance.sqrt();
    
    // Check if the shape is very simple (circle-like) based on circularity
    let is_simple_shape = circularity > 0.85;
    
    // For simple shapes, reduce the variance significantly
    if is_simple_shape {
        // For circle-like shapes, strongly equalize the signal
        // This prevents amplification of tiny variations in simple shapes
        let reduction_factor = 1.0 - ((circularity - 0.85) / 0.15).min(1.0) * 0.95;
        
        paths.iter()
            .map(|&x| mean + (x - mean) * reduction_factor)
            .collect()
    } else {
        // For non-simple shapes, apply moderate conditioning
        // Remove extreme outliers that could artificially inflate entropy
        paths.iter()
            .map(|&x| {
                if std_dev > 1e-10 {
                    let z_score = (x - mean) / std_dev;
                    if z_score.abs() > 3.0 {
                        mean + 3.0 * std_dev * z_score.signum()
                    } else {
                        x
                    }
                } else {
                    x
                }
            })
            .collect()
    }
}

pub fn calculate_spectral_entropy(thornfiddle_paths: &[f64], circularity: f64) -> f64 {
    if thornfiddle_paths.len() < 4 {
        // Not enough data points for meaningful spectral analysis
        return 0.0;
    }

    let original_len = thornfiddle_paths.len();

    // Preprocess paths based on shape properties
    let preprocessed_paths = preprocess_thornfiddle_paths(thornfiddle_paths, circularity);
    
    // Apply Z-score normalization with care
    let normalized_paths = z_score_normalize(&preprocessed_paths);

    // 1. Detrend (remove mean)
    let mean: f64 = normalized_paths.iter().sum::<f64>() / original_len as f64;
    let mut processed_series: Vec<f64> = normalized_paths.iter().map(|&x| x - mean).collect();

    // 2. Apply Hann Window to the actual data
    // For simple shapes (high circularity), use a gentler window
    let window_factor = if circularity > 0.85 { 0.25 } else { 0.5 };
    
    if original_len > 1 {
        for i in 0..original_len {
            let hann_factor = window_factor * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (original_len - 1) as f64).cos());
            processed_series[i] *= hann_factor;
        }
    } else if original_len == 1 {
        processed_series[0] = 0.0;
    }

    // 3. Pad series to a power of 2 for FFT efficiency
    let mut padded_series = processed_series;
    let mut fft_size = 1;
    while fft_size < original_len {
        fft_size *= 2;
    }
    if fft_size < 4 {
        fft_size = 4;
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
    if fft_size < 4 {
        return 0.0;
    }
    
    let power_spectrum: Vec<f64> = complex_input[1..fft_size / 2]
        .iter()
        .map(|c| c.norm_sqr())
        .collect();

    if power_spectrum.is_empty() {
        return 0.0;
    }

    // 7. Normalize to create probability distribution
    let total_power: f64 = power_spectrum.iter().sum();
    if total_power <= 1e-10 {
        return 0.0;
    }

    let probabilities: Vec<f64> = power_spectrum
        .iter()
        .map(|&p| p / total_power)
        .collect();

    // 8. Calculate Shannon entropy
    let entropy: f64 = -probabilities
        .iter()
        .filter(|&&p| p > 1e-10)
        .map(|&p| p * p.log2())
        .sum::<f64>();

    // 9. Normalize by maximum possible entropy
    let num_bins = probabilities.len() as f64;
    if num_bins <= 1.0 {
        return 0.0;
    }

    let max_entropy = num_bins.log2();
    if max_entropy <= 1e-10 {
        entropy
    } else {
        // For circle-like shapes, apply additional scaling to reduce entropy
        let final_entropy = entropy / max_entropy;
        
        if circularity > 0.85 {
            // Apply exponential reduction based on circularity
            // This ensures circles have very low entropy
            let reduction_factor = ((circularity - 0.85) / 0.15).min(1.0);
            final_entropy * (1.0 - reduction_factor * 0.9)
        } else {
            final_entropy
        }
    }
}

/// Create Thornfiddle summary CSV with spectral entropy, circularity, and area
pub fn create_thornfiddle_summary<P: AsRef<Path>>(
    output_dir: P,
    filename: &str,
    subfolder: &str,
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
            "Subfolder",
            "Spectral_Entropy",
            "Circularity",
            "Area",
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data
    writer.write_record(&[
        filename,
        subfolder,
        &format!("{:.6}", spectral_entropy),
        &format!("{:.6}", circularity),
        &area.to_string(),
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

pub fn calculate_features_spectral_entropy(
    features: &[MarginalPointFeatures],
    smoothing_strength: f64,
    circularity: f64
) -> f64 {
    // Extract Thornfiddle path values
    let mut thornfiddle_paths: Vec<f64> = features
        .iter()
        .map(|feature| calculate_thornfiddle_path(feature))
        .collect();
    
    // Apply smoothing if enabled
    if smoothing_strength > 0.0 && thornfiddle_paths.len() > 1 {
        // Convert smoothing strength to window size and sigma
        // Window size should be odd
        let window_size = (smoothing_strength * 2.0 + 1.0).round() as usize;
        // Window size must be at least 3 for meaningful smoothing
        let window_size = window_size.max(3);
        let sigma = smoothing_strength * 0.5;
        
        thornfiddle_paths = apply_gaussian_smoothing(
            &thornfiddle_paths, 
            window_size, 
            sigma
        );
    }
    
    // Calculate spectral entropy with shape awareness
    calculate_spectral_entropy(&thornfiddle_paths, circularity)
}

fn apply_gaussian_smoothing(signal: &[f64], window_size: usize, sigma: f64) -> Vec<f64> {
    if window_size <= 1 || signal.len() <= 1 || sigma <= 0.0 {
        return signal.to_vec();
    }
    
    let half_window = window_size / 2;
    let mut smoothed = Vec::with_capacity(signal.len());
    
    // Create Gaussian kernel
    let mut kernel = Vec::with_capacity(window_size);
    let mut kernel_sum = 0.0;
    
    for i in 0..window_size {
        let x = i as f64 - half_window as f64;
        let weight = (-0.5 * (x / sigma).powi(2)).exp();
        kernel.push(weight);
        kernel_sum += weight;
    }
    
    // Normalize kernel
    for weight in &mut kernel {
        *weight /= kernel_sum;
    }
    
    // Apply convolution
    for i in 0..signal.len() {
        let mut value = 0.0;
        
        for j in 0..window_size {
            let signal_idx = i as isize + j as isize - half_window as isize;
            
            if signal_idx >= 0 && signal_idx < signal.len() as isize {
                value += signal[signal_idx as usize] * kernel[j];
            }
        }
        
        smoothed.push(value);
    }
    
    smoothed
}