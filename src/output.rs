use std::fs;
use std::path::Path;
use csv::Writer;

use crate::errors::{LeafComplexError, Result};
use crate::feature_extraction::MarginalPointFeatures;

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
    
    // Write header
    writer.write_record(&[
        "Point_Index",
        "StraightPath_Length",
        "GyroPath_Length",
        "GyroPath_Perc",
        "CLR_Alpha",
        "CLR_Gamma",
        "GyroPath_Pink",
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
            feature.gyro_path_pink.unwrap_or(0).to_string(),
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    // Flush writer - Fixed: convert io::Error to csv::Error using into()
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}

/// Write LMC (Pink as Transparent) features to CSV
pub fn write_lmc_csv<P: AsRef<Path>>(
    features: &[MarginalPointFeatures],
    output_dir: P,
    filename: &str,
) -> Result<()> {
    let output_path = output_dir.as_ref().join("LMC").join(format!("{}.csv", filename));
    
    // Create directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| LeafComplexError::Io(e))?;
    }
    
    // Create CSV writer
    let mut writer = Writer::from_path(&output_path)
        .map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Write header
    writer.write_record(&[
        "Point_Index",
        "StraightPath_Length",
        "GyroPath_Length",
        "GyroPath_Perc",
        "CLR_Alpha",
        "CLR_Gamma",
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
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    }
    
    // Flush writer - Fixed: convert io::Error to csv::Error using into() 
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}