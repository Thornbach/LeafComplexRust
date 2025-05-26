// src/thornfiddle.rs - Updated with Periodic Gaussian Smoothing and Path-based Spectral Entropy

use std::path::Path;
use std::fs;
use rustfft::{FftPlanner, num_complex::Complex};
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;

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

/// Extract Thornfiddle Path values from features
pub fn extract_thornfiddle_path_signal(features: &[MarginalPointFeatures]) -> Vec<f64> {
    features.iter()
        .map(|feature| calculate_thornfiddle_path(feature))
        .collect()
}

/// Extract Pink Path values from LEC features (DiegoPath_Pink)
pub fn extract_pink_path_signal(features: &[MarginalPointFeatures]) -> Vec<f64> {
    features.iter()
        .map(|feature| feature.diego_path_pink.unwrap_or(0) as f64)
        .collect()
}

/// Calculate spectral entropy from Pink Path signal (WITHOUT smoothing)
pub fn calculate_spectral_entropy_from_pink_path(
    features: &[MarginalPointFeatures]
) -> f64 {
    if features.is_empty() {
        return 0.0;
    }
    
    // Extract Pink Path signal (DiegoPath_Pink values)
    let pink_signal = extract_pink_path_signal(features);
    
    if pink_signal.len() < 4 {
        return 0.0;
    }
    
    // NO SMOOTHING for Pink Path spectral entropy
    
    // Calculate statistics of the raw signal
    let mean = pink_signal.iter().sum::<f64>() / pink_signal.len() as f64;
    let variance = pink_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / pink_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    // Threshold for "simple" signals based on coefficient of variation
    let coefficient_of_variation = if mean > 1e-6 { std_dev / mean } else { 0.0 };
    
    if coefficient_of_variation < 0.01 {
        // Very low variation = very low entropy
        return 0.001 + coefficient_of_variation * 0.01;
    }
    
    if coefficient_of_variation < 0.05 {
        // Low variation = low entropy  
        return 0.01 + coefficient_of_variation * 0.1;
    }
    
    // For signals with significant variation, proceed with spectral analysis
    let powers = calculate_power_spectrum_periodic(&pink_signal);
    if powers.is_empty() {
        return 0.0;
    }
    
    // Calculate Shannon entropy
    let entropy = calculate_shannon_entropy(&powers);
    
    // Scale entropy based on the coefficient of variation
    // More relative variation = higher potential entropy
    let variation_factor = (coefficient_of_variation * 2.0).min(1.0); // Cap at 1.0
    
    entropy * variation_factor
}

/// Calculate Edge Complexity using the provided algorithm
pub fn calculate_edge_feature_density(colored_path_values: &[f64]) -> Result<f64> {
    if colored_path_values.is_empty() {
        return Err(LeafComplexError::Other("Empty colored path values for edge feature density calculation".to_string()));
    }
    
    // Threshold to consider a point as having an edge feature
    let threshold = 1.0;
    
    // Sort values to identify outliers
    let mut sorted_values = colored_path_values.to_vec();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    // Determine outlier threshold (values above 95th percentile are potential outliers)
    let outlier_idx = (sorted_values.len() as f64 * 0.95) as usize;
    let outlier_threshold = if outlier_idx < sorted_values.len() {
        sorted_values[outlier_idx]
    } else {
        f64::MAX
    };
    
    println!("Outlier threshold (95th percentile): {}", outlier_threshold);
    
    // Step 1: Find all connected sequences of edge features
    let mut feature_sequences = Vec::new();
    let mut current_sequence = Vec::new();
    let mut has_extreme_value = false;
    
    // We'll wrap around the contour to handle sequences that cross the start/end boundary
    let extended_values: Vec<f64> = colored_path_values.iter()
        .chain(colored_path_values.iter())
        .cloned()
        .collect();
    
    for i in 0..colored_path_values.len() {
        let value = extended_values[i];
        
        if value > threshold {
            // Add point to current sequence
            current_sequence.push((i, value));
            
            // Check if this is an extreme value (in top 5%)
            if value >= outlier_threshold {
                has_extreme_value = true;
            }
        } else if !current_sequence.is_empty() {
            // End of sequence - store if it contains at least one extreme value
            if has_extreme_value {
                feature_sequences.push(current_sequence.clone());
            }
            // Reset for next sequence
            current_sequence.clear();
            has_extreme_value = false;
        }
    }
    
    // Don't forget to add the last sequence if it's not empty
    if !current_sequence.is_empty() && has_extreme_value {
        feature_sequences.push(current_sequence);
    }
    
    // Step 2: Identify the longest sequence (likely the petiole)
    let longest_sequence = feature_sequences.iter()
        .max_by_key(|seq| seq.len())
        .cloned();
    
    // Create a filtered copy of the values with the longest sequence set to zero
    let mut filtered_values = colored_path_values.to_vec();
    
    if let Some(petiole_sequence) = longest_sequence {
        println!("Identified potential petiole sequence with {} points", petiole_sequence.len());
        
        // Set all values in the longest sequence to 0
        for (idx, _) in petiole_sequence {
            // Make sure we handle wrapped indices correctly
            let actual_idx = idx % colored_path_values.len();
            filtered_values[actual_idx] = 0.0;
        }
    } else {
        println!("No significant petiole sequence identified");
    }
    
    // Step 3: Calculate complexity metrics on the filtered values
    
    // Count points with significant edge features (after filtering)
    let feature_points = filtered_values.iter()
        .filter(|&&v| v > threshold)
        .count();
    
    // Total number of contour points
    let total_points = filtered_values.len();
    
    // Sum the magnitudes of edge features (after filtering)
    let sum_feature_magnitudes = filtered_values.iter()
        .filter(|&&v| v > threshold)
        .sum::<f64>();
    
    if total_points > 0 {
        // Calculate feature density (proportion of contour with edge features)
        let density = feature_points as f64 / total_points as f64;
        
        // Calculate average magnitude of edge features
        let avg_magnitude = if feature_points > 0 {
            sum_feature_magnitudes / feature_points as f64
        } else {
            0.0
        };
        
        // Combined metric emphasizing both presence and size of features
        // Note: Using a scaling factor of 10.0 since LEC_SCALING_FACTOR isn't accessible here
        let edge_complexity = density * (1.0 + avg_magnitude.sqrt()) * 10.0;
                
        println!("Feature density: {:.4}, Avg magnitude: {:.4}, Edge complexity: {:.4}", 
                 density, avg_magnitude, edge_complexity);
        
        Ok(edge_complexity)
    } else {
        Ok(0.0)
    }
}

/// Apply periodic-aware Gaussian smoothing to a signal
/// The signal is treated as periodic (last point connects to first point)
pub fn periodic_gaussian_smooth(signal: &[f64], window_size: usize, sigma: f64) -> Vec<f64> {
    if signal.len() < 3 || window_size == 0 {
        return signal.to_vec();
    }
    
    let n = signal.len();
    let mut smoothed = Vec::with_capacity(n);
    
    // Generate Gaussian weights
    let half_window = window_size / 2;
    let mut weights = Vec::with_capacity(window_size);
    let mut weight_sum = 0.0;
    
    for i in 0..window_size {
        let offset = i as f64 - half_window as f64;
        let weight = (-0.5 * (offset / sigma).powi(2)).exp();
        weights.push(weight);
        weight_sum += weight;
    }
    
    // Normalize weights
    for weight in &mut weights {
        *weight /= weight_sum;
    }
    
    // Apply smoothing with periodic boundary conditions
    for i in 0..n {
        let mut weighted_sum = 0.0;
        
        for j in 0..window_size {
            let offset = j as i32 - half_window as i32;
            let idx = ((i as i32 + offset) + n as i32) % n as i32;
            let idx = if idx < 0 { idx + n as i32 } else { idx } as usize;
            
            weighted_sum += signal[idx] * weights[j];
        }
        
        smoothed.push(weighted_sum);
    }
    
    smoothed
}

/// Calculate power spectrum using FFT for periodic signal
fn calculate_power_spectrum_periodic(signal: &[f64]) -> Vec<f64> {
    if signal.len() < 4 {
        return Vec::new();
    }
    
    // Check if signal has significant variation
    let mean = signal.iter().sum::<f64>() / signal.len() as f64;
    let variance = signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / signal.len() as f64;
    
    if variance < 1e-6 {
        return Vec::new(); // No variation = no complexity
    }
    
    // For periodic signals, we don't need padding - use the signal as-is
    let mut fft_size = signal.len();
    
    // Convert to complex numbers (remove DC component for better frequency analysis)
    let mut complex_input: Vec<Complex<f64>> = signal
        .iter()
        .map(|&x| Complex::new(x - mean, 0.0))
        .collect();
    
    // Extend to next power of 2 for efficiency if needed
    if !fft_size.is_power_of_two() {
        fft_size = fft_size.next_power_of_two();
        complex_input.resize(fft_size, Complex::new(0.0, 0.0));
    }
    
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

/// Calculate spectral entropy from Thornfiddle Path signal
pub fn calculate_spectral_entropy_from_thornfiddle_path(
    features: &[MarginalPointFeatures],
    smoothing_strength: f64
) -> (f64, Vec<f64>) {
    if features.is_empty() {
        return (0.0, Vec::new());
    }
    
    // Extract Thornfiddle Path signal
    let thornfiddle_signal = extract_thornfiddle_path_signal(features);
    
    if thornfiddle_signal.len() < 4 {
        return (0.0, thornfiddle_signal);
    }
    
    // Apply periodic-aware Gaussian smoothing
    let window_size = (thornfiddle_signal.len() / 8).max(3).min(21); // Adaptive window size
    let sigma = smoothing_strength.max(0.5); // Ensure minimum sigma
    let smoothed_signal = periodic_gaussian_smooth(&thornfiddle_signal, window_size, sigma);
    
    // Calculate statistics of the smoothed signal
    let mean = smoothed_signal.iter().sum::<f64>() / smoothed_signal.len() as f64;
    let variance = smoothed_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / smoothed_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    // Threshold for "simple" signals based on coefficient of variation
    let coefficient_of_variation = if mean > 1e-6 { std_dev / mean } else { 0.0 };
    
    if coefficient_of_variation < 0.01 {
        // Very low variation = very low entropy
        return (0.001 + coefficient_of_variation * 0.01, smoothed_signal);
    }
    
    if coefficient_of_variation < 0.05 {
        // Low variation = low entropy  
        return (0.01 + coefficient_of_variation * 0.1, smoothed_signal);
    }
    
    // For signals with significant variation, proceed with spectral analysis
    let powers = calculate_power_spectrum_periodic(&smoothed_signal);
    if powers.is_empty() {
        return (0.0, smoothed_signal);
    }
    
    // Calculate Shannon entropy
    let entropy = calculate_shannon_entropy(&powers);
    
    // Scale entropy based on the coefficient of variation
    // More relative variation = higher potential entropy
    let variation_factor = (coefficient_of_variation * 2.0).min(1.0); // Cap at 1.0
    
    (entropy * variation_factor, smoothed_signal)
}

/// Create Thornfiddle summary CSV with both circularity scores and new metrics
pub fn create_thornfiddle_summary<P: AsRef<Path>>(
    output_dir: P,
    filename: &str,
    subfolder: &str,
    spectral_entropy: f64,
    spectral_entropy_pink: f64,
    edge_complexity: f64,
    lec_circularity: f64,
    lmc_circularity: f64,
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
            "Spectral_Entropy_Pink",
            "Edge_Complexity",
            "LEC_Circularity",
            "LMC_Circularity",
            "Area",
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data
    writer.write_record(&[
        filename,
        subfolder,
        &format!("{:.6}", spectral_entropy),
        &format!("{:.6}", spectral_entropy_pink),
        &format!("{:.6}", edge_complexity),
        &format!("{:.6}", lec_circularity),
        &format!("{:.6}", lmc_circularity),
        &area.to_string(),
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

/// Debug Thornfiddle values with smoothed path information
pub fn debug_thornfiddle_values(
    features: &[MarginalPointFeatures],
    filename: &str,
    output_dir: &Path,
    spectral_entropy: f64,
    smoothed_thornfiddle_path: &[f64],
    smoothing_strength: f64,
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
        "Thornfiddle_Path_Smoothed",
        "Spectral_Entropy",
        "Smoothing_Strength",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write data for each point
    for (i, feature) in features.iter().enumerate() {
        let path_ratio = if feature.straight_path_length > 0.0 {
            feature.diego_path_length / feature.straight_path_length
        } else {
            1.0
        };
        
        let multiplier = calculate_thornfiddle_multiplier(feature);
        let thornfiddle_path = calculate_thornfiddle_path(feature);
        let smoothed_value = smoothed_thornfiddle_path.get(i).copied().unwrap_or(0.0);
        
        writer.write_record(&[
            &feature.point_index.to_string(),
            &format!("{:.6}", feature.straight_path_length),
            &format!("{:.6}", feature.diego_path_length),
            &format!("{:.6}", path_ratio),
            &format!("{:.6}", multiplier),
            &format!("{:.6}", thornfiddle_path),
            &format!("{:.6}", smoothed_value),
            &format!("{:.6}", spectral_entropy),
            &format!("{:.6}", smoothing_strength),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    Ok(())
}