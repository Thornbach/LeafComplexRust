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
    lec_length: f64,          // LEC biological length
    lec_width: f64,           // LEC biological width
    lec_shape_index: f64,     // NEW: LEC Shape Index
    lmc_length: f64,          // NEW: LMC biological length
    lmc_width: f64,           // NEW: LMC biological width  
    lmc_shape_index: f64,     // NEW: LMC Shape Index
    dynamic_opening_percentage: f64, // NEW: Dynamic opening percentage used
    outline_count: u32,       // outline point count
    harmonic_chain_count: usize, // number of valid harmonic chains
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
        
        // Write header only for new file - UPDATED with shape index fields
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
            "LEC_Length",          // LEC biological length
            "LEC_Width",           // LEC biological width
            "LEC_ShapeIndex",      // NEW: LEC Shape Index
            "LMC_Length",          // NEW: LMC biological length
            "LMC_Width",           // NEW: LMC biological width
            "LMC_ShapeIndex",      // NEW: LMC Shape Index
            "Dynamic_Opening_Percentage", // NEW: Dynamic opening percentage
            "Outline_Count",
            "Harmonic_Chain_Count", // number of valid harmonic chains
        ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
        
        writer
    };
    
    // Write data - UPDATED with shape index fields
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
        &format!("{:.1}", lec_length),    // LEC biological length
        &format!("{:.1}", lec_width),     // LEC biological width
        &format!("{:.3}", lec_shape_index), // NEW: LEC Shape Index
        &format!("{:.1}", lmc_length),    // NEW: LMC biological length
        &format!("{:.1}", lmc_width),     // NEW: LMC biological width
        &format!("{:.3}", lmc_shape_index), // NEW: LMC Shape Index
        &format!("{:.1}", dynamic_opening_percentage), // NEW: Dynamic opening percentage
        &outline_count.to_string(),
        &harmonic_chain_count.to_string(), // harmonic chain count
    ]).map_err(|e| LeafComplexError::CsvOutput(e))?;
    
    // Flush writer
    writer.flush().map_err(|e| LeafComplexError::CsvOutput(csv::Error::from(e)))?;
    
    Ok(())
}