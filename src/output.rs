// Enhanced src/output.rs - Updated with new weighted chain metrics

use std::fs;
use std::path::Path;
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;
use crate::thornfiddle::{calculate_thornfiddle_multiplier, calculate_thornfiddle_path};

/// Write LEC (Pink as Opaque) features to CSV
pub fn write_lec_csv<P: AsRef<Path>>(
    features: &[MarginalPointFeatures],
    output_dir: P,
    filename: &str,
) -> Result<()> {
    let output_path = output_dir.as_ref().join("LEC").join(format!("{}.csv", filename));
    
    // Create directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| LeafComplexError::Io(e))?;
    }
    
    // Create CSV writer
    let mut writer = Writer::from_path(&output_path)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write header - include DiegoPath and Harmonic fields
    writer.write_record(&[
        "Point_Index",
        "StraightPath_Length",
        "GyroPath_Length",
        "GyroPath_Perc",
        "CLR_Alpha",
        "CLR_Gamma",
        "Left_CLR_Alpha",
        "Left_CLR_Gamma",
        "Right_CLR_Alpha",
        "Right_CLR_Gamma",
        "DiegoPath_Length",
        "DiegoPath_Perc",
        "DiegoPath_Pink",
        "Thornfiddle_Path_Harmonic",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write data
    for feature in features {
        writer.write_record(&[
            feature.point_index.to_string(),
            format!("{:.6}", feature.straight_path_length),
            format!("{:.6}", feature.gyro_path_length),
            format!("{:.6}", feature.gyro_path_perc),
            feature.clr_alpha.to_string(),
            feature.clr_gamma.to_string(),
            feature.left_clr_alpha.to_string(),
            feature.left_clr_gamma.to_string(),
            feature.right_clr_alpha.to_string(),
            feature.right_clr_gamma.to_string(),
            format!("{:.6}", feature.diego_path_length),
            format!("{:.6}", feature.diego_path_perc),
            feature.diego_path_pink.unwrap_or(0).to_string(),
            format!("{:.6}", feature.thornfiddle_path_harmonic),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

pub fn write_lmc_csv<P: AsRef<Path>>(
    features: &[MarginalPointFeatures],
    output_dir: P,
    filename: &str,
    smoothed_thornfiddle_path: Option<&[f64]>,
) -> Result<()> {
    let output_path = output_dir.as_ref().join("LMC").join(format!("{}.csv", filename));
    
    // Create directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| LeafComplexError::Io(e))?;
    }
    
    // Create CSV writer
    let mut writer = Writer::from_path(&output_path)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write header - include DiegoPath, Thornfiddle, Thornfiddle_Path_Smoothed, and Harmonic fields
    writer.write_record(&[
        "Point_Index",
        "StraightPath_Length",
        "GyroPath_Length",
        "GyroPath_Perc",
        "CLR_Alpha",
        "CLR_Gamma",
        "Left_CLR_Alpha",
        "Left_CLR_Gamma",
        "Right_CLR_Alpha",
        "Right_CLR_Gamma",
        "DiegoPath_Length",
        "DiegoPath_Perc",
        "Thornfiddle_Multiplier",
        "Thornfiddle_Path",
        "Thornfiddle_Path_Smoothed",
        "Thornfiddle_Path_Harmonic",
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write data
    for (i, feature) in features.iter().enumerate() {
        // Calculate Thornfiddle values for this feature
        let thornfiddle_multiplier = calculate_thornfiddle_multiplier(feature);
        let thornfiddle_path = calculate_thornfiddle_path(feature);
        
        // Get smoothed value if available
        let thornfiddle_path_smoothed = if let Some(smoothed) = smoothed_thornfiddle_path {
            smoothed.get(i).copied().unwrap_or(thornfiddle_path)
        } else {
            thornfiddle_path // Fallback to unsmoothed if no smoothed data
        };
        
        writer.write_record(&[
            feature.point_index.to_string(),
            format!("{:.6}", feature.straight_path_length),
            format!("{:.6}", feature.gyro_path_length),
            format!("{:.6}", feature.gyro_path_perc),
            feature.clr_alpha.to_string(),
            feature.clr_gamma.to_string(),
            feature.left_clr_alpha.to_string(),
            feature.left_clr_gamma.to_string(),
            feature.right_clr_alpha.to_string(),
            feature.right_clr_gamma.to_string(),
            format!("{:.6}", feature.diego_path_length),
            format!("{:.6}", feature.diego_path_perc),
            format!("{:.6}", thornfiddle_multiplier),
            format!("{:.6}", thornfiddle_path),
            format!("{:.6}", thornfiddle_path_smoothed),
            format!("{:.6}", feature.thornfiddle_path_harmonic),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

/// ENHANCED: Create Thornfiddle summary CSV with new weighted chain metrics
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
    weighted_chain_score: f64,     // NEW: Chain intensity weighting
    rhythm_regularity: f64,        // NEW: Rhythm regularity factor
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
        
        // ENHANCED: Write header with new weighted chain metrics
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
            "Weighted_Chain_Score",    // NEW: Chain intensity scoring
            "Rhythm_Regularity",       // NEW: Rhythm regularity factor
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // ENHANCED: Write data with new weighted chain metrics
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
        &format!("{:.2}", weighted_chain_score),    // NEW: 2 decimal places for chain intensity
        &format!("{:.3}", rhythm_regularity),       // NEW: 3 decimal places for regularity
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}