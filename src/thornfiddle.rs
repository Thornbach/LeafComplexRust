// src/thornfiddle.rs - DiegoPath-Enhanced Spectral Entropy

use std::path::Path;
use std::fs;
use rustfft::{FftPlanner, num_complex::Complex};
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;
use crate::morphology::{trace_contour, resample_contour, smooth_contour};

/// Calculate a more aggressive Thornfiddle Multiplier that better captures complexity
pub fn calculate_thornfiddle_multiplier(feature: &MarginalPointFeatures) -> f64 {
    // Skip calculation if DiegoPath or StraightPath is invalid
    if feature.straight_path_length <= 0.0 || feature.diego_path_length <= 0.0 {
        return 1.0;
    }
    
    // Calculate path deviations - how much the path deviates from straight line
    let path_ratio = feature.diego_path_length / feature.straight_path_length;
    
    // Use exponential scaling to amplify differences
    // This will create much stronger separation between simple and complex leaves
    let path_complexity = ((path_ratio - 1.0) * 4.0).exp() - 1.0;
    
    // Apply upper limit to avoid extreme values
    let path_complexity = path_complexity.min(15.0);
    
    // Calculate Region_Factor based on transparent vs non-transparent regions
    // This captures how much the path has to navigate around transparent areas
    let clr_sum = (feature.clr_alpha + feature.clr_gamma) as f64;
    let denominator = feature.straight_path_length + 1.0;
    let region_factor = (clr_sum / denominator).sqrt() * 2.0; // Amplify region factor
    
    // Calculate final multiplier with increased sensitivity
    let multiplier = 1.0 + path_complexity * (1.0 + region_factor);
    
    // Apply non-linear scaling to further separate simple from complex
    multiplier.powf(1.2)
}

/// Calculate enhanced Thornfiddle Path
pub fn calculate_thornfiddle_path(feature: &MarginalPointFeatures) -> f64 {
    let multiplier = calculate_thornfiddle_multiplier(feature);
    feature.diego_path_length * multiplier
}

/// Extract contour signature with DiegoPath weighting
fn extract_diegopath_weighted_signature(
    contour: &[(u32, u32)],
    features: &[MarginalPointFeatures],
    interpolation_points: usize
) -> Vec<f64> {
    // First, check if we have enough contour points and features
    if contour.len() < 8 || features.is_empty() {
        return Vec::new();
    }
    
    // Resample contour to fixed number of points
    let resampled = resample_contour(contour, interpolation_points);
    
    // Calculate centroid (for reference)
    let n = resampled.len() as f64;
    let sum_x: f64 = resampled.iter().map(|&(x, _)| x as f64).sum();
    let sum_y: f64 = resampled.iter().map(|&(_, y)| y as f64).sum();
    
    let centroid_x = sum_x / n;
    let centroid_y = sum_y / n;
    
    // Calculate basic distance signature
    let mut distances: Vec<f64> = resampled.iter()
        .map(|&(x, y)| {
            let dx = x as f64 - centroid_x;
            let dy = y as f64 - centroid_y;
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    
    // Normalize by average radius
    let avg_radius = distances.iter().sum::<f64>() / n;
    if avg_radius > 0.0 {
        for d in &mut distances {
            *d /= avg_radius;
        }
    }
    
    // Now enhance the signature using DiegoPath information
    // Calculate the average DiegoPath/StraightPath ratio
    let avg_ratio = features.iter()
        .filter(|f| f.straight_path_length > 0.0)
        .map(|f| f.diego_path_length / f.straight_path_length)
        .sum::<f64>() / features.len() as f64;
    
    // Calculate the enhancement factor
    // Higher ratios indicate more complex leaves and get more enhancement
    let enhancement = (avg_ratio - 1.0).max(0.0) * 5.0 + 1.0;
    
    // Generate thornfiddle path values
    let thornfiddle_values: Vec<f64> = features.iter()
        .map(|f| calculate_thornfiddle_path(f))
        .collect();
    
    // Find average thornfiddle path
    let avg_thornfiddle = thornfiddle_values.iter().sum::<f64>() / thornfiddle_values.len() as f64;
    
    // Normalize thornfiddle values
    let normalized_thornfiddle: Vec<f64> = thornfiddle_values.iter()
        .map(|&v| v / avg_thornfiddle)
        .collect();
    
    // Enhance the distance signature with the thornfiddle information
    let mut enhanced_signature = Vec::with_capacity(distances.len());
    
    for (i, &d) in distances.iter().enumerate() {
        // Map contour index to feature index (using modulo if sizes differ)
        let feature_idx = (i * features.len()) / distances.len();
        
        // Get corresponding normalized thornfiddle value
        let thornfiddle_factor = normalized_thornfiddle[feature_idx % normalized_thornfiddle.len()];
        
        // Enhance the distance with the thornfiddle factor and general enhancement
        let enhanced_value = d * (1.0 + (thornfiddle_factor - 1.0) * enhancement);
        
        enhanced_signature.push(enhanced_value);
    }
    
    enhanced_signature
}

/// Calculate power spectrum from signature
fn calculate_power_spectrum(signature: &[f64]) -> Vec<f64> {
    if signature.len() < 8 {
        return Vec::new();
    }
    
    // 1. Normalize signature to zero mean
    let mean = signature.iter().sum::<f64>() / signature.len() as f64;
    let mut normalized = signature.iter().map(|&x| x - mean).collect::<Vec<f64>>();
    
    // 2. Apply a Hann window to reduce spectral leakage
    for i in 0..normalized.len() {
        let window_factor = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / 
                                         (normalized.len() - 1) as f64).cos());
        normalized[i] *= window_factor;
    }
    
    // 3. Pad to power of 2 for FFT efficiency
    let mut fft_size = 1;
    while fft_size < normalized.len() {
        fft_size *= 2;
    }
    
    let mut padded = normalized;
    padded.resize(fft_size, 0.0);
    
    // 4. Perform FFT
    let mut complex_input: Vec<Complex<f64>> = padded
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut complex_input);
    
    // 5. Calculate power spectrum (excluding DC component)
    let mut powers = Vec::with_capacity(fft_size / 2);
    
    // Skip DC component (index 0)
    for i in 1..fft_size / 2 {
        powers.push(complex_input[i].norm_sqr());
    }
    
    // 6. Normalize powers to sum to 1
    let total_power: f64 = powers.iter().sum();
    if total_power > 0.0 {
        for p in &mut powers {
            *p /= total_power;
        }
    }
    
    powers
}

/// Calculate Shannon entropy from power spectrum
fn calculate_spectral_entropy(powers: &[f64], circularity: f64) -> f64 {
    if powers.is_empty() {
        return 0.0;
    }
    
    // 1. Calculate basic Shannon entropy
    let entropy = -powers.iter()
        .filter(|&&p| p > 1e-10)
        .map(|&p| p * p.log2())
        .sum::<f64>();
    
    // 2. Normalize by maximum possible entropy
    let max_entropy = (powers.len() as f64).log2();
    let normalized_entropy = if max_entropy > 1e-10 {
        entropy / max_entropy
    } else {
        0.0
    };
    
    // 3. Adjust based on circularity - circles should have low entropy regardless
    let circularity_factor = if circularity > 0.95 {
        0.001  // Perfect circles
    } else if circularity > 0.8 {
        0.2    // Near circles
    } else if circularity > 0.6 {
        0.5    // Somewhat circular
    } else if circularity > 0.4 {
        0.8    // Moderately complex
    } else {
        1.0    // Highly complex
    };
    
    // 4. Apply circularity adjustment
    let adjusted_entropy = normalized_entropy * circularity_factor;
    
    // 5. Apply non-linear scaling to enhance separation
    adjusted_entropy.powf(0.7)
}

/// Calculate DiegoPath-enhanced spectral entropy
fn calculate_diegopath_spectral_entropy(
    contour: &[(u32, u32)],
    features: &[MarginalPointFeatures],
    circularity: f64,
    interpolation_points: usize
) -> f64 {
    if contour.len() < 8 || features.is_empty() {
        return 0.0;
    }
    
    // Force very circular shapes to have minimal entropy
    if circularity > 0.95 {
        return 0.001;
    }
    
    // 1. Extract DiegoPath-weighted signature
    let signature = extract_diegopath_weighted_signature(
        contour,
        features,
        interpolation_points
    );
    
    // 2. Calculate power spectrum
    let powers = calculate_power_spectrum(&signature);
    
    // 3. Calculate spectral entropy
    let entropy = calculate_spectral_entropy(&powers, circularity);
    
    // 4. Scale based on average Diego/Straight ratio to add extra separation
    let avg_ratio = features.iter()
        .filter(|f| f.straight_path_length > 0.0)
        .map(|f| f.diego_path_length / f.straight_path_length)
        .sum::<f64>() / features.len() as f64;
    
    // Higher ratios imply more complex shapes and should have higher entropy
    let diego_boost = if avg_ratio > 1.5 {
        // Significant deviation - strong boost
        1.5
    } else if avg_ratio > 1.2 {
        // Moderate deviation - medium boost
        1.3
    } else if avg_ratio > 1.1 {
        // Slight deviation - small boost
        1.1
    } else {
        // Minimal deviation - no boost
        1.0
    };
    
    // Apply the boost and ensure result is in [0,1]
    (entropy * diego_boost).min(1.0)
}

/// Calculate spectral entropy using DiegoPath information
pub fn calculate_contour_spectral_entropy(
    contour: &[(u32, u32)], 
    features: &[MarginalPointFeatures],
    circularity: f64,
    interpolation_points: usize
) -> f64 {
    calculate_diegopath_spectral_entropy(
        contour,
        features,
        circularity,
        interpolation_points
    )
}

/// API-compatible function that gets DiegoPath info from MarginalPointFeatures
pub fn calculate_features_spectral_entropy(
    features: &[MarginalPointFeatures],
    _smoothing_strength: f64,
    circularity: f64,
    _area: u32,
    interpolation_points: usize
) -> f64 {
    // Create a dummy contour from feature indices
    // In real pipeline usage, the actual contour will be passed separately
    let dummy_contour: Vec<(u32, u32)> = features.iter()
        .map(|f| (f.point_index as u32, f.straight_path_length as u32))
        .collect();
    
    // Calculate spectral entropy
    calculate_contour_spectral_entropy(
        &dummy_contour,
        features,
        circularity,
        interpolation_points
    )
}

/// Create Thornfiddle summary CSV with spectral entropy and circularity
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

/// Detailed DiegoPath signature analysis
pub fn debug_diegopath_analysis<P: AsRef<Path>>(
    contour: &[(u32, u32)],
    features: &[MarginalPointFeatures],
    filename: &str,
    output_dir: P,
    circularity: f64,
    interpolation_points: usize
) -> Result<()> {
    let debug_dir = output_dir.as_ref().join("debug");
    fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
    
    let debug_file = debug_dir.join(format!("{}_diegopath_analysis.csv", filename));
    let mut writer = Writer::from_path(debug_file)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Extract signature data
    let signature = extract_diegopath_weighted_signature(contour, features, interpolation_points);
    let powers = calculate_power_spectrum(&signature);
    let entropy = calculate_spectral_entropy(&powers, circularity);
    
    // Calculate average Diego/Straight ratio
    let avg_ratio = features.iter()
        .filter(|f| f.straight_path_length > 0.0)
        .map(|f| f.diego_path_length / f.straight_path_length)
        .sum::<f64>() / features.len() as f64;
    
    // Calculate average thornfiddle multiplier
    let avg_multiplier = features.iter()
        .map(|f| calculate_thornfiddle_multiplier(f))
        .sum::<f64>() / features.len() as f64;
    
    // Write header
    writer.write_record(&[
        "Filename",
        "Circularity",
        "Avg_DiegoRatio",
        "Avg_Multiplier",
        "Raw_Entropy",
        "Final_Entropy",
        "Num_Features",
        "Num_Contour_Points",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Calculate final entropy
    let final_entropy = calculate_diegopath_spectral_entropy(
        contour,
        features,
        circularity,
        interpolation_points
    );
    
    // Write data
    writer.write_record(&[
        filename,
        &format!("{:.6}", circularity),
        &format!("{:.6}", avg_ratio),
        &format!("{:.6}", avg_multiplier),
        &format!("{:.6}", entropy),
        &format!("{:.6}", final_entropy),
        &features.len().to_string(),
        &contour.len().to_string(),
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    Ok(())
}

/// Basic debug function
pub fn debug_thornfiddle_values(
    features: &[MarginalPointFeatures],
    filename: &str,
    output_dir: &Path,
    circularity: f64,
    area: u32,
    interpolation_points: usize
) -> Result<()> {
    let debug_dir = output_dir.join("debug");
    fs::create_dir_all(&debug_dir).map_err(|e| LeafComplexError::Io(e))?;
    
    let debug_file = debug_dir.join(format!("{}_thornfiddle_debug.csv", filename));
    let mut writer = Writer::from_path(debug_file)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write header
    writer.write_record(&[
        "Point_Index",
        "StraightPath",
        "DiegoPath",
        "DiegoPath_Ratio",
        "Thornfiddle_Multiplier",
        "Thornfiddle_Path",
        "Circularity",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write data for each point
    for feature in features {
        let path_ratio = if feature.straight_path_length > 0.0 {
            feature.diego_path_length / feature.straight_path_length
        } else {
            1.0
        };
        
        let multiplier = calculate_thornfiddle_multiplier(feature);
        let thornfiddle_path = feature.diego_path_length * multiplier;
        
        writer.write_record(&[
            &feature.point_index.to_string(),
            &format!("{:.6}", feature.straight_path_length),
            &format!("{:.6}", feature.diego_path_length),
            &format!("{:.6}", path_ratio),
            &format!("{:.6}", multiplier),
            &format!("{:.6}", thornfiddle_path),
            &format!("{:.6}", circularity),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    Ok(())
}