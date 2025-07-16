// Enhanced src/thornfiddle.rs - Final 10% improvements with chain intensity, frequency weighting, and rhythm analysis

use std::path::Path;
use std::fs;
use std::collections::HashSet;
use rustfft::{FftPlanner, num_complex::Complex};
use csv::Writer;
use std::f64::consts::PI;
use image::RgbaImage;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;
use crate::image_utils::has_rgb_color;
use crate::path_algorithms::trace_straight_line;

/// Represents a chain of consecutive golden pixel crossings (lobes)
#[derive(Debug, Clone)]
struct GoldenChain {
    start_index: usize,
    end_index: usize,
    length: usize,
    total_golden_pixels: u32,
    max_crossing_count: u32,
}

/// ENHANCED: Result structure containing harmonic values, chain statistics, and new weighted metrics
#[derive(Debug)]
pub struct HarmonicResult {
    pub harmonic_values: Vec<f64>,
    pub valid_chain_count: usize,
    pub total_chain_count: usize,
    pub weighted_chain_score: f64,  // NEW: Chain intensity weighting
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
    if mean_deviation < 5.0 {
        return 0.001 + mean_deviation * 0.0001;
    }
    
    if max_deviation < 10.0 {
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
    let magnitude_factor = (mean_deviation / 20.0).min(1.0);
    
    entropy * magnitude_factor
}

/// Detect petiole sequence in a signal using outlier analysis
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
            current_sequence.push(i);
            
            if value >= outlier_threshold {
                has_extreme_value = true;
            }
        } else if !current_sequence.is_empty() {
            if has_extreme_value {
                feature_sequences.push(current_sequence.clone());
            }
            current_sequence.clear();
            has_extreme_value = false;
        }
    }
    
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
        // Mode 1: Set petiole values to zero
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

/// Filter petiole from LEC features with optional threshold filtering
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
        let pink_signal = extract_pink_path_signal(&working_features);
        
        petiole_indices = detect_petiole_sequence(&pink_signal, threshold);
        
        if let Some(ref indices) = petiole_indices {
            if remove_completely {
                let petiole_set: HashSet<usize> = indices.iter().cloned().collect();
                let filtered_features: Vec<MarginalPointFeatures> = working_features.iter()
                    .enumerate()
                    .filter(|(i, _)| !petiole_set.contains(i))
                    .map(|(_, feature)| feature.clone())
                    .collect();
                
                working_features = filtered_features.into_iter()
                    .enumerate()
                    .map(|(new_idx, mut feature)| {
                        feature.point_index = new_idx;
                        feature
                    })
                    .collect();
                
                println!("LEC petiole removal: {} -> {} features", features.len(), working_features.len());
            } else {
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
    
    let threshold = 1.0;
    
    let filtered_values = if enable_petiole_filter {
        if let Some(petiole_indices) = detect_petiole_sequence(colored_path_values, threshold) {
            apply_petiole_filter(colored_path_values, &petiole_indices, petiole_remove_completely)
        } else {
            colored_path_values.to_vec()
        }
    } else {
        colored_path_values.to_vec()
    };
    
    let feature_points = filtered_values.iter()
        .filter(|&&v| v > threshold)
        .count();
    
    let total_points = filtered_values.len();
    
    let sum_feature_magnitudes = filtered_values.iter()
        .filter(|&&v| v > threshold)
        .sum::<f64>();
    
    if total_points > 0 {
        let density = feature_points as f64 / total_points as f64;
        
        let avg_magnitude = if feature_points > 0 {
            sum_feature_magnitudes / feature_points as f64
        } else {
            0.0
        };
        
        let edge_complexity = density * (1.0 + avg_magnitude.sqrt()) * scaling_factor;          

        println!("Feature density: {:.4}, Avg magnitude: {:.4}, Edge complexity: {:.4}", 
                 density, avg_magnitude, edge_complexity);
        
        Ok(edge_complexity)
    } else {
        Ok(0.0)
    }
}

/// Apply periodic-aware Gaussian smoothing to a signal
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

// Removed rhythm regularity and frequency band weighting functions

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
        return Vec::new();
    }
    
    let mut fft_size = signal.len();
    
    // Convert to complex numbers (remove DC component)
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
        .filter(|&&p| p > 1e-12)
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

/// Calculate spectral entropy from Harmonic Thornfiddle Path (simplified - removed rhythm and frequency weighting)
pub fn calculate_spectral_entropy_from_harmonic_thornfiddle_path(
    features: &[MarginalPointFeatures],
    smoothing_strength: f64
) -> (f64, Vec<f64>) {
    if features.is_empty() {
        return (0.0, Vec::new());
    }
    
    // Extract Harmonic Thornfiddle Path signal
    let harmonic_signal = extract_harmonic_thornfiddle_path_signal(features);
    
    if harmonic_signal.len() < 4 {
        return (0.0, harmonic_signal);
    }
    
    // Apply periodic-aware Gaussian smoothing
    let window_size = (harmonic_signal.len() / 8).max(3).min(21);
    let sigma = smoothing_strength.max(0.5);
    let smoothed_signal = periodic_gaussian_smooth(&harmonic_signal, window_size, sigma);
    
    // Calculate statistics of the smoothed signal
    let mean = smoothed_signal.iter().sum::<f64>() / smoothed_signal.len() as f64;
    let variance = smoothed_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / smoothed_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    let coefficient_of_variation = if mean > 1e-6 { std_dev / mean } else { 0.0 };
    
    if coefficient_of_variation < 0.01 {
        return (0.001 + coefficient_of_variation * 0.01, smoothed_signal);
    }
    
    if coefficient_of_variation < 0.05 {
        return (0.01 + coefficient_of_variation * 0.1, smoothed_signal);
    }
    
    // For signals with significant variation, proceed with basic spectral analysis
    let powers = calculate_power_spectrum_periodic(&smoothed_signal);
    if powers.is_empty() {
        return (0.0, smoothed_signal);
    }
    
    // Calculate Shannon entropy from basic spectrum
    let entropy = calculate_shannon_entropy(&powers);
    
    // Scale entropy based on coefficient of variation
    let variation_factor = (coefficient_of_variation * 2.0).min(1.0);
    let final_entropy = entropy * variation_factor;
    
    (final_entropy, smoothed_signal)
}

/// Legacy spectral entropy function (maintained for backward compatibility)
pub fn calculate_spectral_entropy_from_thornfiddle_path(
    features: &[MarginalPointFeatures],
    smoothing_strength: f64
) -> (f64, Vec<f64>) {
    let (entropy, smoothed) = calculate_spectral_entropy_from_harmonic_thornfiddle_path(features, smoothing_strength);
    (entropy, smoothed)
}

/// Calculate approximate entropy from Pink Path
pub fn calculate_approximate_entropy_from_pink_path(
    features: &[MarginalPointFeatures],
    m: usize,
    r: f64,
) -> f64 {
    if features.is_empty() {
        return 0.0;
    }
    
    let pink_signal = extract_pink_path_signal(features);
    
    if pink_signal.len() < 4 {
        return 0.0;
    }
    
    let mean = pink_signal.iter().sum::<f64>() / pink_signal.len() as f64;
    let variance = pink_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / pink_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    let adaptive_r = if std_dev > 1e-6 {
        r * std_dev
    } else {
        r
    };
    
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

/// Create Thornfiddle summary CSV with weighted chain metrics (removed rhythm regularity)
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
    lec_length: f64,
    lec_width: f64,
    lec_shape_index: f64,
    lmc_length: f64,
    lmc_width: f64,
    lmc_shape_index: f64,
    dynamic_opening_percentage: f64,
    dynamic_kernel_size: u32,
    outline_count: u32,
    harmonic_chain_count: usize,
    weighted_chain_score: f64,     // Chain intensity weighting
) -> Result<()> {
    let thornfiddle_dir = output_dir.as_ref().join("Thornfiddle");
    fs::create_dir_all(&thornfiddle_dir).map_err(|e| LeafComplexError::Io(e))?;
    
    let summary_path = thornfiddle_dir.join("summary.csv");
    
    let file_exists = summary_path.exists();
    
    let mut writer = if file_exists {
        Writer::from_writer(fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&summary_path)
            .map_err(|e| LeafComplexError::Io(e))?)
    } else {
        let mut writer = Writer::from_path(&summary_path)
            .map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        // Write header with weighted chain metrics (removed rhythm regularity)
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
            "LEC_Length",
            "LEC_Width",
            "LEC_ShapeIndex",
            "LMC_Length",
            "LMC_Width",
            "LMC_ShapeIndex",
            "Dynamic_Opening_Percentage",
            "Dynamic_Kernel_Size",
            "Outline_Count",
            "Harmonic_Chain_Count",
            "Weighted_Chain_Score",    // Chain intensity scoring
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data with weighted chain metrics (removed rhythm regularity)
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
        &format!("{:.1}", lec_length),
        &format!("{:.1}", lec_width),
        &format!("{:.3}", lec_shape_index),
        &format!("{:.1}", lmc_length),
        &format!("{:.1}", lmc_width),
        &format!("{:.3}", lmc_shape_index),
        &format!("{:.1}", dynamic_opening_percentage),
        &dynamic_kernel_size.to_string(),
        &outline_count.to_string(),
        &harmonic_chain_count.to_string(),
        &format!("{:.2}", weighted_chain_score),    // Chain intensity weighting
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

/// Calculate a simple Thornfiddle Multiplier based on path complexity
pub fn calculate_thornfiddle_multiplier(feature: &MarginalPointFeatures) -> f64 {
    if feature.straight_path_length <= 0.0 || feature.diego_path_length <= 0.0 {
        return 1.0;
    }
    
    let path_ratio = feature.diego_path_length / feature.straight_path_length;
    let base_multiplier = path_ratio.max(1.0);
    let clr_factor = (feature.clr_alpha + feature.clr_gamma) as f64 / 1000.0;
    
    base_multiplier + clr_factor.min(0.5)
}

/// Calculate Thornfiddle Path with simple multiplier
pub fn calculate_thornfiddle_path(feature: &MarginalPointFeatures) -> f64 {
    let multiplier = calculate_thornfiddle_multiplier(feature);
    feature.diego_path_length * multiplier
}

/// ENHANCED: Calculate Thornfiddle Path with Golden Pixel Harmonic enhancement and chain intensity weighting
pub fn calculate_thornfiddle_path_harmonic(
    features: &[MarginalPointFeatures],
    leaf_circumference: f64,
    thornfiddle_image: &RgbaImage,
    reference_point: (u32, u32),
    contour_points: &[(u32, u32)],
    golden_color: [u8; 3],
    pixel_threshold: u32,
    min_chain_length: usize,
    harmonic_strength_multiplier: f64,
) -> HarmonicResult {
    if features.is_empty() {
        return HarmonicResult {
            harmonic_values: Vec::new(),
            valid_chain_count: 0,
            total_chain_count: 0,
            weighted_chain_score: 0.0,
        };
    }
    
    println!("Calculating harmonic Thornfiddle with golden pixel detection");
    println!("Parameters: pixel_threshold={}, min_chain_length={}, harmonic_strength={}",
             pixel_threshold, min_chain_length, harmonic_strength_multiplier);
    
    // Step 1: Detect golden chains based on pixel crossings
    let golden_chains = detect_golden_chains(
        features,
        thornfiddle_image,
        reference_point,
        contour_points,
        golden_color,
        pixel_threshold,
    );
    
    let total_chain_count = golden_chains.len();
    
    // Step 2: Filter chains by minimum length requirement
    let valid_chains: Vec<&GoldenChain> = golden_chains.iter()
        .filter(|chain| chain.length >= min_chain_length)
        .collect();
    
    let valid_chain_count = valid_chains.len();
    
    // NEW: Step 2.5: Calculate weighted chain score (chain intensity weighting)
    let weighted_chain_score: f64 = valid_chains.iter()
        .map(|chain| (chain.total_golden_pixels as f64) * (chain.length as f64))
        .sum();
    
    println!("Detected {} total chains, {} valid chains (>= {} points), weighted score: {:.1}", 
             total_chain_count, valid_chain_count, min_chain_length, weighted_chain_score);
    
    // Step 3: Calculate base Thornfiddle values
    let base_thornfiddle: Vec<f64> = features.iter()
        .map(|feature| calculate_thornfiddle_path(feature))
        .collect();
    
    // Step 4: Calculate global complexity based on VALID chains only
    let global_complexity = calculate_global_golden_complexity(&valid_chains.iter().cloned().cloned().collect::<Vec<_>>());
    
    // Step 5: Apply harmonics to each point using VALID chains only
    let mut harmonic_thornfiddle = base_thornfiddle.clone();
    
    for (chain_index, &chain) in valid_chains.iter().enumerate() {
        apply_golden_chain_harmonics(
            &mut harmonic_thornfiddle,
            chain,
            chain_index,
            global_complexity,
            leaf_circumference,
            features,
            valid_chain_count,
            harmonic_strength_multiplier,
        );
    }
    
    // Step 6: Handle additive effects for overlapping VALID chains
    apply_additive_harmonic_effects(&mut harmonic_thornfiddle, &valid_chains.iter().cloned().cloned().collect::<Vec<_>>());
    
    println!("Harmonic enhancement complete - {} valid chains processed, weighted score: {:.1}", 
             valid_chain_count, weighted_chain_score);
    
    HarmonicResult {
        harmonic_values: harmonic_thornfiddle,
        valid_chain_count,
        total_chain_count,
        weighted_chain_score,  // NEW: Include weighted score
    }
}

/// Count golden pixels crossed by a path
fn count_golden_pixels_crossed(
    path: &[(u32, u32)],
    thornfiddle_image: &RgbaImage,
    golden_color: [u8; 3],
) -> u32 {
    let mut golden_count = 0;
    let (width, height) = thornfiddle_image.dimensions();
    
    for &(x, y) in path {
        if x < width && y < height {
            let pixel = thornfiddle_image.get_pixel(x, y);
            if has_rgb_color(pixel, golden_color) {
                golden_count += 1;
            }
        }
    }
    
    golden_count
}

/// Detect chains of consecutive golden pixel crossings
fn detect_golden_chains(
    features: &[MarginalPointFeatures],
    thornfiddle_image: &RgbaImage,
    reference_point: (u32, u32),
    contour_points: &[(u32, u32)],
    golden_color: [u8; 3],
    pixel_threshold: u32,
) -> Vec<GoldenChain> {
    let mut chains = Vec::new();
    let mut current_chain_start: Option<usize> = None;
    let mut chain_golden_counts = Vec::new();
    
    for (i, feature) in features.iter().enumerate() {
        if i >= contour_points.len() {
            break;
        }
        
        let marginal_point = contour_points[i];
        
        let path_to_check = if feature.diego_path_perc > 101.0 {
            trace_straight_line(reference_point, marginal_point)
        } else {
            trace_straight_line(reference_point, marginal_point)
        };
        
        let golden_count = count_golden_pixels_crossed(&path_to_check, thornfiddle_image, golden_color);
        let crosses_threshold = golden_count >= pixel_threshold;
        
        if crosses_threshold {
            if current_chain_start.is_none() {
                current_chain_start = Some(i);
                chain_golden_counts.clear();
            }
            chain_golden_counts.push(golden_count);
        } else if let Some(start) = current_chain_start {
            if !chain_golden_counts.is_empty() {
                let total_golden_pixels: u32 = chain_golden_counts.iter().sum();
                let max_crossing_count = *chain_golden_counts.iter().max().unwrap_or(&0);
                
                chains.push(GoldenChain {
                    start_index: start,
                    end_index: i - 1,
                    length: i - start,
                    total_golden_pixels,
                    max_crossing_count,
                });
                
                println!("Golden chain detected: indices {}-{}, length {}, total golden pixels {}", 
                         start, i - 1, i - start, total_golden_pixels);
            }
            current_chain_start = None;
            chain_golden_counts.clear();
        }
    }
    
    // Handle chain that extends to end of contour
    if let Some(start) = current_chain_start {
        if !chain_golden_counts.is_empty() {
            let total_golden_pixels: u32 = chain_golden_counts.iter().sum();
            let max_crossing_count = *chain_golden_counts.iter().max().unwrap_or(&0);
            
            chains.push(GoldenChain {
                start_index: start,
                end_index: features.len() - 1,
                length: features.len() - start,
                total_golden_pixels,
                max_crossing_count,
            });
            
            println!("Golden chain detected (end): indices {}-{}, length {}, total golden pixels {}", 
                     start, features.len() - 1, features.len() - start, total_golden_pixels);
        }
    }
    
    chains
}

/// Calculate global complexity factor with isolation bonus
fn calculate_global_golden_complexity(chains: &[GoldenChain]) -> f64 {
    if chains.is_empty() {
        return 0.0;
    }
    
    let total_chain_length: usize = chains.iter().map(|c| c.length).sum();
    let total_golden_pixels: u32 = chains.iter().map(|c| c.total_golden_pixels).sum();
    let avg_golden_intensity: f64 = total_golden_pixels as f64 / chains.len() as f64;
    
    let isolation_factor = if chains.len() <= 2 {
        1.0
    } else {
        1.0 + (chains.len() as f64 - 2.0) * 0.3
    };
    
    let chain_count_factor = (chains.len() as f64).ln() + 1.0;
    
    chain_count_factor * (total_chain_length as f64).sqrt() * avg_golden_intensity.sqrt() * isolation_factor
}

/// Apply harmonic enhancement with configurable strength and isolation effects
fn apply_golden_chain_harmonics(
    harmonic_values: &mut [f64],
    chain: &GoldenChain,
    chain_index: usize,
    global_complexity: f64,
    leaf_circumference: f64,
    features: &[MarginalPointFeatures],
    _total_valid_chains: usize,
    harmonic_strength_multiplier: f64,
) {
    let max_harmonics = calculate_max_harmonics(chain.length);
    let base_frequency = calculate_base_frequency(leaf_circumference, features.len());
    let accumulated_stress = calculate_accumulated_stress(chain_index, global_complexity);
    
    for i in chain.start_index..=chain.end_index {
        if i >= harmonic_values.len() {
            break;
        }
        
        let chain_position = i - chain.start_index;
        let position_ratio = chain_position as f64 / chain.length as f64;
        
        let golden_intensity = chain.total_golden_pixels as f64 / chain.length as f64;
        
        let chaos_factor = calculate_golden_chaos_factor_monotonic(
            position_ratio,
            accumulated_stress,
            golden_intensity,
            chain.max_crossing_count as f64,
        );
        
        let harmonic_component = generate_harmonic_component(
            chain_position,
            max_harmonics,
            base_frequency,
            chaos_factor,
            position_ratio,
        );
        
        let enhanced_harmonic = harmonic_component * harmonic_strength_multiplier;
        
        let base_value = harmonic_values[i];
        harmonic_values[i] = base_value + (base_value * enhanced_harmonic);
    }
}

/// Calculate maximum harmonics using Natural Overtone Series
fn calculate_max_harmonics(chain_length: usize) -> usize {
    if chain_length == 0 {
        return 0;
    }
    
    let log_component = (chain_length as f64).log2().floor() as usize;
    (log_component + 3).max(1).min(12)
}

/// Calculate base frequency proportional to leaf circumference
fn calculate_base_frequency(circumference: f64, contour_points: usize) -> f64 {
    if circumference <= 0.0 || contour_points == 0 {
        return 1.0;
    }
    
    let normalized_circumference = circumference / contour_points as f64;
    2.0 / (1.0 + normalized_circumference / 10.0)
}

/// Calculate accumulated stress from previous chains
fn calculate_accumulated_stress(chain_index: usize, global_complexity: f64) -> f64 {
    let chain_stress = (chain_index as f64 * 0.3).tanh();
    chain_stress * global_complexity
}

/// Calculate chaos factor using MONOTONIC progression
fn calculate_golden_chaos_factor_monotonic(
    position_ratio: f64,
    accumulated_stress: f64,
    golden_intensity: f64,
    max_golden_count: f64,
) -> f64 {
    let log_progression = (1.0 + position_ratio * 9.0).ln() / 10.0_f64.ln();
    let smooth_progression = 0.8 + position_ratio * 0.4;
    
    let golden_factor = if max_golden_count > 0.0 {
        golden_intensity / max_golden_count
    } else {
        1.0
    };
    
    let base_chaos = log_progression * smooth_progression * golden_factor;
    let stress_enhanced = base_chaos * (1.0 + accumulated_stress);
    
    stress_enhanced.min(2.0)
}

/// Generate harmonic component using natural overtone series
fn generate_harmonic_component(
    position: usize,
    max_harmonics: usize,
    base_frequency: f64,
    chaos_factor: f64,
    position_ratio: f64,
) -> f64 {
    if max_harmonics == 0 {
        return 0.0;
    }
    
    let mut harmonic_sum = 0.0;
    
    let seed = (position as f64 * 1000.0 + base_frequency * 100.0) as u64;
    let mut rng_state = seed;
    
    for harmonic_index in 1..=max_harmonics {
        let frequency = base_frequency * harmonic_index as f64;
        let amplitude = 1.0 / harmonic_index as f64;
        
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let phase_offset = (rng_state as f64 / u64::MAX as f64) * PI * 2.0;
        
        let harmonic_value = amplitude * chaos_factor * 
            (frequency * position_ratio * PI * 2.0 + phase_offset).sin();
        
        harmonic_sum += harmonic_value;
    }
    
    harmonic_sum / max_harmonics as f64
}

/// Apply additive effects for overlapping chains
fn apply_additive_harmonic_effects(
    harmonic_values: &mut [f64],
    chains: &[GoldenChain],
) {
    let mut point_chain_counts = vec![0usize; harmonic_values.len()];
    
    for chain in chains {
        for i in chain.start_index..=chain.end_index {
            if i < point_chain_counts.len() {
                point_chain_counts[i] += 1;
            }
        }
    }
    
    for (i, &chain_count) in point_chain_counts.iter().enumerate() {
        if chain_count > 1 && i < harmonic_values.len() {
            let additive_factor = 1.0 + (chain_count as f64 - 1.0) * 0.3;
            harmonic_values[i] *= additive_factor;
        }
    }
}

/// Calculate leaf circumference from contour points
pub fn calculate_leaf_circumference(contour: &[(u32, u32)]) -> f64 {
    if contour.len() < 2 {
        return 0.0;
    }
    
    let mut circumference = 0.0;
    for i in 0..contour.len() {
        let current = contour[i];
        let next = contour[(i + 1) % contour.len()];
        
        let dx = next.0 as f64 - current.0 as f64;
        let dy = next.1 as f64 - current.1 as f64;
        circumference += (dx * dx + dy * dy).sqrt();
    }
    
    circumference
}

/// Extract Thornfiddle Path values from features
pub fn extract_thornfiddle_path_signal(features: &[MarginalPointFeatures]) -> Vec<f64> {
    features.iter()
        .map(|feature| calculate_thornfiddle_path(feature))
        .collect()
}

/// Extract Harmonic Thornfiddle Path values from features  
pub fn extract_harmonic_thornfiddle_path_signal(features: &[MarginalPointFeatures]) -> Vec<f64> {
    features.iter()
        .map(|feature| feature.thornfiddle_path_harmonic)
        .collect()
}

/// Extract Pink Path values from LEC features
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
    
    let pink_signal = extract_pink_path_signal(features);
    
    if pink_signal.len() < 4 {
        return 0.0;
    }
    
    let mean = pink_signal.iter().sum::<f64>() / pink_signal.len() as f64;
    let variance = pink_signal.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / pink_signal.len() as f64;
    let std_dev = variance.sqrt();
    
    let coefficient_of_variation = if mean > 1e-6 { std_dev / mean } else { 0.0 };
    
    if coefficient_of_variation < 0.01 {
        return 0.001 + coefficient_of_variation * 0.01;
    }
    
    if coefficient_of_variation < 0.05 {
        return 0.01 + coefficient_of_variation * 0.1;
    }
    
    let powers = calculate_power_spectrum_periodic(&pink_signal);
    if powers.is_empty() {
        return 0.0;
    }
    
    let entropy = calculate_shannon_entropy(&powers);
    let variation_factor = (coefficient_of_variation * 2.0).min(1.0);
    
    entropy * variation_factor
}