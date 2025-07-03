// Updated src/output.rs - Added Thornfiddle_Path_Harmonic column

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
    
    // Flush writer - Fixed: convert io::Error to csv::Error using into()
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