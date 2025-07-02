// src/thornfiddle.rs - Updated create_thornfiddle_summary function

use std::path::Path;
use std::fs;
use std::collections::HashSet;
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

/// Extract contour signature using absolute distance deviations from mean radius
fn extract_contour_signature(contour: &[(u32, u32)], interpolation_points: usize) -> Vec<f64> {
    use crate::morphology::resample_contour;
    
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

/// Calculate spectral entropy from contour with magnitude-based thresholding
pub fn calculate_spectral_entropy_from_contour(
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
    let powers = calculate_power_spectrum_periodic(&signature);
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

/// Detect petiole sequence in a signal using outlier analysis
/// Returns the indices of the detected petiole sequence (longest sequence with extreme values)
pub fn detect_petiole_sequence(signal: &[f64], threshold: f64) -> Option<Vec<usize>> {
    if signal.is_empty() {
        return None;
    }
    
    // Sort values to identify outliers
    let mut sorted_values = signal.to_vec();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    // Determine outlier threshold (values above 95th percentile are potential outliers)
    let outlier_idx = (sorted_values.len() as f64 * 0.95) as usize;
    let outlier_threshold = if outlier_idx < sorted_values.len() {
        sorted_values[outlier_idx]
    } else {
        f64::MAX
    };
    
    println!("Petiole detection - Outlier threshold (95th percentile): {}", outlier_threshold);
    
    // Find all connected sequences of features above threshold
    let mut feature_sequences = Vec::new();
    let mut current_sequence = Vec::new();
    let mut has_extreme_value = false;
    
    // We'll wrap around the contour to handle sequences that cross the start/end boundary
    let extended_signal: Vec<f64> = signal.iter()
        .chain(signal.iter())
        .cloned()
        .collect();
    
    for i in 0..signal.len() {
        let value = extended_signal[i];
        
        if value > threshold {
            // Add point to current sequence
            current_sequence.push(i);
            
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
    
    // Find the longest sequence (likely the petiole)
    let longest_sequence = feature_sequences.into_iter()
        .max_by_key(|seq| seq.len());
    
    if let Some(petiole_sequence) = longest_sequence {
        println!("Detected petiole sequence with {} points", petiole_sequence.len());
        Some(petiole_sequence)
    } else {
        println!("No significant petiole sequence detected");
        None
    }
}

/// Apply petiole filter to a signal
/// Mode: true = set to zero, false = remove completely and merge ends
pub fn apply_petiole_filter(signal: &[f64], petiole_indices: &[usize], remove_completely: bool) -> Vec<f64> {
    if petiole_indices.is_empty() {
        return signal.to_vec();
    }
    
    if remove_completely {
        // Mode 2: Remove petiole completely and merge loose ends
        let mut filtered_signal = Vec::new();
        let petiole_set: std::collections::HashSet<usize> = petiole_indices.iter().cloned().collect();
        
        for (i, &value) in signal.iter().enumerate() {
            if !petiole_set.contains(&i) {
                filtered_signal.push(value);
            }
        }
        
        println!("Petiole removal: {} -> {} points", signal.len(), filtered_signal.len());
        filtered_signal
    } else {
        // Mode 1: Set petiole values to zero (current behavior)
        let mut filtered_signal = signal.to_vec();
        for &idx in petiole_indices {
            if idx < filtered_signal.len() {
                filtered_signal[idx] = 0.0;
            }
        }
        
        println!("Petiole zeroing: {} points set to zero", petiole_indices.len());
        filtered_signal
    }
}

/// Apply threshold filter to pink path values
/// Sets all values at or below the threshold to zero
pub fn apply_pink_threshold_filter(
    features: &mut [MarginalPointFeatures],
    enable_threshold_filter: bool,
    threshold: f64,
) {
    if !enable_threshold_filter {
        return;
    }
    
    let mut filtered_count = 0;
    
    for feature in features.iter_mut() {
        if let Some(pink_value) = feature.diego_path_pink {
            if (pink_value as f64) <= threshold {
                feature.diego_path_pink = Some(0);
                filtered_count += 1;
            }
        }
    }
    
    if filtered_count > 0 {
        println!("Pink threshold filter: {} values <= {:.1} set to zero", filtered_count, threshold);
    }
}

/// Filter petiole from LEC features (Pink Path signal) with optional threshold filtering
pub fn filter_petiole_from_lec_features(
    features: &[MarginalPointFeatures],
    enable_petiole_filter: bool,
    remove_completely: bool,
    threshold: f64,
    enable_pink_threshold_filter: bool,
    pink_threshold: f64,
) -> (Vec<MarginalPointFeatures>, Option<Vec<usize>>) {
    if !enable_petiole_filter && !enable_pink_threshold_filter {
        return (features.to_vec(), None);
    }
    
    let mut working_features = features.to_vec();
    let mut petiole_indices = None;
    
    // Step 1: Apply petiole filtering if enabled
    if enable_petiole_filter && !working_features.is_empty() {
        // Extract pink path signal for petiole detection
        let pink_signal = extract_pink_path_signal(&working_features);
        
        // Detect petiole sequence
        petiole_indices = detect_petiole_sequence(&pink_signal, threshold);
        
        if let Some(ref indices) = petiole_indices {
            if remove_completely {
                // Remove features at petiole indices completely
                let petiole_set: HashSet<usize> = indices.iter().cloned().collect();
                let filtered_features: Vec<MarginalPointFeatures> = working_features.iter()
                    .enumerate()
                    .filter(|(i, _)| !petiole_set.contains(i))
                    .map(|(_, feature)| feature.clone())
                    .collect();
                
                // Update point indices to be sequential
                working_features = filtered_features.into_iter()
                    .enumerate()
                    .map(|(new_idx, mut feature)| {
                        feature.point_index = new_idx;
                        feature
                    })
                    .collect();
                
                println!("LEC petiole removal: {} -> {} features", features.len(), working_features.len());
            } else {
                // Set petiole features' pink values to zero but keep all features
                for &idx in indices {
                    if idx < working_features.len() {
                        working_features[idx].diego_path_pink = Some(0);
                    }
                }
                
                println!("LEC petiole zeroing: {} features modified", indices.len());
            }
        }
    }
    
    // Step 2: Apply pink threshold filtering if enabled
    apply_pink_threshold_filter(&mut working_features, enable_pink_threshold_filter, pink_threshold);
    
    (working_features, petiole_indices)
}

/// Calculate Edge Complexity using the provided algorithm with configurable petiole filtering
pub fn calculate_edge_feature_density(
    colored_path_values: &[f64],
    enable_petiole_filter: bool,
    petiole_remove_completely: bool,
    scaling_factor: f64,
) -> Result<f64> {
    if colored_path_values.is_empty() {
        return Err(LeafComplexError::Other("Empty colored path values for edge feature density calculation".to_string()));
    }
    
    // Threshold to consider a point as having an edge feature
    let threshold = 1.0;
    
    // Apply petiole filtering if enabled
    let filtered_values = if enable_petiole_filter {
        if let Some(petiole_indices) = detect_petiole_sequence(colored_path_values, threshold) {
            apply_petiole_filter(colored_path_values, &petiole_indices, petiole_remove_completely)
        } else {
            colored_path_values.to_vec()
        }
    } else {
        colored_path_values.to_vec()
    };
    
    // Calculate complexity metrics on the (potentially filtered) values
    
    // Count points with significant edge features
    let feature_points = filtered_values.iter()
        .filter(|&&v| v > threshold)
        .count();
    
    // Total number of contour points
    let total_points = filtered_values.len();
    
    // Sum the magnitudes of edge features
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
        let edge_complexity = density * (1.0 + avg_magnitude.sqrt()) * scaling_factor;          

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

pub fn calculate_approximate_entropy_from_pink_path(
    features: &[MarginalPointFeatures],
    m: usize,
    r: f64,
) -> f64 {
    if features.is_empty() {
        return 0.0;
    }
    
    // Extract Pink Path signal (DiegoPath_Pink values)
    let pink_signal = extract_pink_path_signal(features);
    
    if pink_signal.len() < 4 {
        return 0.0;
    }
    
    // NO SMOOTHING for Pink Path approximate entropy (same as spectral entropy approach)
    
    // Calculate statistics of the raw signal for adaptive tolerance
    let mean = pink_signal.iter().sum::<f64>() / pink_signal.len() as f64;
    let variance = pink_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / pink_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    // Adaptive tolerance based on signal characteristics
    let adaptive_r = if std_dev > 1e-6 {
        r * std_dev // Scale tolerance by signal variability
    } else {
        r // Use fixed tolerance for very low variation signals
    };
    
    // Calculate approximate entropy
    calculate_approximate_entropy(&pink_signal, m, adaptive_r)
}

/// Calculate approximate entropy for a given signal
pub fn calculate_approximate_entropy(signal: &[f64], m: usize, r: f64) -> f64 {
    let n = signal.len();
    if n <= m {
        return 0.0;
    }
    
    let phi_m = calculate_phi(signal, m, r);
    let phi_m1 = calculate_phi(signal, m + 1, r);
    
    phi_m - phi_m1
}

/// Helper function to calculate phi for approximate entropy
fn calculate_phi(signal: &[f64], m: usize, r: f64) -> f64 {
    let n = signal.len();
    let mut sum = 0.0;
    
    for i in 0..=(n - m) {
        let mut matches = 0;
        
        for j in 0..=(n - m) {
            let max_diff = calculate_max_distance(&signal[i..i+m], &signal[j..j+m]);
            
            if max_diff <= r {
                matches += 1;
            }
        }
        
        let ratio = matches as f64 / (n - m + 1) as f64;
        if ratio > 1e-12 {
            sum += ratio.ln();
        }
    }
    
    sum / (n - m + 1) as f64
}

/// Calculate maximum distance between two pattern vectors
fn calculate_max_distance(pattern1: &[f64], pattern2: &[f64]) -> f64 {
    pattern1.iter()
        .zip(pattern2.iter())
        .map(|(&a, &b)| (a - b).abs())
        .fold(0.0, |acc, diff| acc.max(diff))
}

/// Create Thornfiddle summary CSV with circularity scores, biological dimensions, and outline count
/// Updated to use biological length/width instead of bounding box dimensions
pub fn create_thornfiddle_summary<P: AsRef<Path>>(
    output_dir: P,
    filename: &str,
    subfolder: &str,
    spectral_entropy: f64,
    spectral_entropy_pink: f64,
    spectral_entropy_contour: f64,
    approximate_entropy: f64,
    edge_complexity: f64,
    lec_circularity: f64,
    lmc_circularity: f64,
    area: u32,
    length: f64,          // NEW: biological length (longest distance between contour points)
    width: f64,           // NEW: biological width (perpendicular to length axis)
    outline_count: u32,   // outline point count
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
        
        // Write header only for new file - UPDATED with biological dimension headers
        writer.write_record(&[
            "ID",
            "Subfolder",
            "Spectral_Entropy",
            "Spectral_Entropy_Pink",
            "Spectral_Entropy_Contour",
            "Approximate_Entropy",
            "Edge_Complexity",
            "LEC_Circularity",
            "LMC_Circularity",
            "Area",
            "Length",          // CHANGED from "Width" to "Length" (biological)
            "Width",           // CHANGED from "Height" to "Width" (biological)
            "Outline_Count",
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data - UPDATED with biological dimensions
    writer.write_record(&[
        filename,
        subfolder,
        &format!("{:.6}", spectral_entropy),
        &format!("{:.6}", spectral_entropy_pink),
        &format!("{:.6}", spectral_entropy_contour),
        &format!("{:.6}", approximate_entropy),
        &format!("{:.6}", edge_complexity),
        &format!("{:.6}", lec_circularity),
        &format!("{:.6}", lmc_circularity),
        &area.to_string(),
        &format!("{:.1}", length),    // CHANGED: now biological length
        &format!("{:.1}", width),     // CHANGED: now biological width  
        &outline_count.to_string(),
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}