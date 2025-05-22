// src/thornfiddle.rs - Simplified Spectral Entropy Calculation

use std::path::Path;
use std::fs;
use rustfft::{FftPlanner, num_complex::Complex};
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;
use crate::morphology::{resample_contour};

/// Calculate a simple Thornfiddle Multiplier based on path complexity
pub fn calculate_thornfiddle_multiplier(feature: &MarginalPointFeatures) -> f64 {
    if feature.straight_path_length <= 0.0 || feature.diego_path_length <= 0.0 {
        return 1.0;
    }
    
    // Simple ratio-based multiplier
    let path_ratio = feature.diego_path_length / feature.straight_path_length;
    
    // Basic multiplier: more complex paths get higher multipliers
    let base_multiplier = path_ratio.max(1.0);
    
    // Add small contribution from CLR regions
    let clr_factor = (feature.clr_alpha + feature.clr_gamma) as f64 / 1000.0;
    
    base_multiplier + clr_factor.min(0.5)
}

/// Calculate Thornfiddle Path with simple multiplier
pub fn calculate_thornfiddle_path(feature: &MarginalPointFeatures) -> f64 {
    let multiplier = calculate_thornfiddle_multiplier(feature);
    feature.diego_path_length * multiplier
}

/// Extract contour signature using absolute distance deviations from mean radius
fn extract_contour_signature(contour: &[(u32, u32)], interpolation_points: usize) -> Vec<f64> {
    if contour.len() < 3 {
        return Vec::new();
    }
    
    // Resample contour to fixed number of points
    let resampled = resample_contour(contour, interpolation_points);
    if resampled.is_empty() {
        return Vec::new();
    }
    
    // Calculate centroid
    let n = resampled.len() as f64;
    let sum_x: f64 = resampled.iter().map(|&(x, _)| x as f64).sum();
    let sum_y: f64 = resampled.iter().map(|&(_, y)| y as f64).sum();
    
    let centroid_x = sum_x / n;
    let centroid_y = sum_y / n;
    
    // Calculate distances from centroid
    let distances: Vec<f64> = resampled.iter()
        .map(|&(x, y)| {
            let dx = x as f64 - centroid_x;
            let dy = y as f64 - centroid_y;
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    
    // Apply light smoothing to reduce digitization noise
    let smoothed_distances = smooth_signal(&distances, 2);
    
    // Calculate mean radius
    let mean_radius = smoothed_distances.iter().sum::<f64>() / n;
    
    // Return ABSOLUTE deviations from mean radius
    // This preserves the actual magnitude of shape complexity
    let absolute_deviations: Vec<f64> = smoothed_distances.iter()
        .map(|&d| (d - mean_radius).abs())
        .collect();
    
    absolute_deviations
}

/// Simple smoothing filter to reduce noise
fn smooth_signal(signal: &[f64], window_size: usize) -> Vec<f64> {
    if signal.len() < 3 || window_size == 0 {
        return signal.to_vec();
    }
    
    let mut smoothed = Vec::with_capacity(signal.len());
    let half_window = window_size / 2;
    
    for i in 0..signal.len() {
        let mut sum = 0.0;
        let mut count = 0;
        
        // Calculate window bounds
        let start = if i >= half_window { i - half_window } else { 0 };
        let end = std::cmp::min(i + half_window + 1, signal.len());
        
        // Average over window
        for j in start..end {
            sum += signal[j];
            count += 1;
        }
        
        smoothed.push(sum / count as f64);
    }
    
    smoothed
}

/// Calculate power spectrum using FFT with proper scaling for absolute values
fn calculate_power_spectrum(signature: &[f64]) -> Vec<f64> {
    if signature.len() < 4 {
        return Vec::new();
    }
    
    // For absolute deviations, we don't remove mean (it's already deviation from mean)
    // Just ensure we have some signal
    let max_value = signature.iter().fold(0.0f64, |a, &b| a.max(b));
    if max_value < 1e-6 {
        return Vec::new(); // Nearly no variation = no complexity
    }
    
    // Pad to next power of 2 for efficiency
    let mut fft_size = 1;
    while fft_size < signature.len() {
        fft_size *= 2;
    }
    
    let mut padded = signature.to_vec();
    padded.resize(fft_size, 0.0);
    
    // Convert to complex numbers
    let mut complex_input: Vec<Complex<f64>> = padded
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    
    // Perform FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut complex_input);
    
    // Calculate power spectrum (magnitude squared)
    let mut powers = Vec::with_capacity(fft_size / 2);
    
    // Skip DC component (index 0) and use only positive frequencies
    for i in 1..fft_size / 2 {
        powers.push(complex_input[i].norm_sqr());
    }
    
    // Normalize so powers sum to 1
    let total_power: f64 = powers.iter().sum();
    if total_power > 0.0 {
        for p in &mut powers {
            *p /= total_power;
        }
    }
    
    powers
}

/// Calculate Shannon entropy from normalized power spectrum
fn calculate_shannon_entropy(powers: &[f64]) -> f64 {
    if powers.is_empty() {
        return 0.0;
    }
    
    // Calculate Shannon entropy: -Σ(p * log2(p))
    let entropy = -powers.iter()
        .filter(|&&p| p > 1e-12) // Avoid log(0)
        .map(|&p| p * p.log2())
        .sum::<f64>();
    
    // Normalize by maximum possible entropy
    let max_entropy = (powers.len() as f64).log2();
    if max_entropy > 1e-6 {
        entropy / max_entropy
    } else {
        0.0
    }
}

/// Calculate spectral entropy from contour with magnitude-based thresholding
pub fn calculate_spectral_entropy(
    contour: &[(u32, u32)], 
    interpolation_points: usize
) -> f64 {
    // Extract contour signature
    let signature = extract_contour_signature(contour, interpolation_points);
    if signature.is_empty() {
        return 0.0;
    }
    
    // Calculate statistics of absolute deviations
    let mean_deviation = signature.iter().sum::<f64>() / signature.len() as f64;
    let max_deviation = signature.iter().fold(0.0f64, |a, &b| a.max(b));
    
    // Threshold for "simple" shapes based on absolute deviation magnitude
    // If mean absolute deviation is very small, it's essentially a circle/simple shape
    if mean_deviation < 5.0 {
        // Very low variation = very low entropy
        return 0.001 + mean_deviation * 0.0001; // Tiny entropy proportional to variation
    }
    
    if max_deviation < 10.0 {
        // Low variation = low entropy  
        return 0.01 + mean_deviation * 0.001;
    }
    
    // For shapes with significant variation, proceed with spectral analysis
    let powers = calculate_power_spectrum(&signature);
    if powers.is_empty() {
        return 0.0;
    }
    
    // Calculate Shannon entropy
    let entropy = calculate_shannon_entropy(&powers);
    
    // Scale entropy based on the magnitude of deviations
    // More absolute variation = higher potential entropy
    let magnitude_factor = (mean_deviation / 20.0).min(1.0); // Cap at 1.0
    
    entropy * magnitude_factor
}

/// Create Thornfiddle summary CSV
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

/// Debug Thornfiddle values (simplified)
pub fn debug_thornfiddle_values(
    features: &[MarginalPointFeatures],
    filename: &str,
    output_dir: &Path,
    spectral_entropy: f64,
) -> Result<()> {
    let debug_dir = output_dir.join("debug");
    fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
    
    let debug_file = debug_dir.join(format!("{}_thornfiddle_debug.csv", filename));
    let mut writer = Writer::from_path(debug_file)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write header
    writer.write_record(&[
        "Point_Index",
        "StraightPath_Length",
        "DiegoPath_Length",
        "Path_Ratio",
        "Thornfiddle_Multiplier",
        "Thornfiddle_Path",
        "Spectral_Entropy",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write data for each point
    for feature in features {
        let path_ratio = if feature.straight_path_length > 0.0 {
            feature.diego_path_length / feature.straight_path_length
        } else {
            1.0
        };
        
        let multiplier = calculate_thornfiddle_multiplier(feature);
        let thornfiddle_path = calculate_thornfiddle_path(feature);
        
        writer.write_record(&[
            &feature.point_index.to_string(),
            &format!("{:.6}", feature.straight_path_length),
            &format!("{:.6}", feature.diego_path_length),
            &format!("{:.6}", path_ratio),
            &format!("{:.6}", multiplier),
            &format!("{:.6}", thornfiddle_path),
            &format!("{:.6}", spectral_entropy),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    Ok(())
}