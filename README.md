# LeafComplexR

LeafComplexR is a sophisticated Rust application for analyzing leaf morphology from PNG images with transparency. The program provides comprehensive analysis of leaf features through advanced image processing, geometric analysis, and path-based feature extraction.

## Key Features

- **Advanced Image Processing**
  - High-performance morphological operations
  - Contour tracing and analysis
  - Support for transparent PNG images
  - Configurable image preprocessing

- **Complex Analysis Algorithms**
  - Golden spiral path generation
  - CLR (Complex Leaf Region) calculation
  - LEC (Leaf Edge Complexity) analysis
  - LMC (Leaf Margin Complexity) analysis
  - Spectral entropy analysis
  - Approximate entropy analysis
  - Point-based feature extraction

- **User Interface**
  - Interactive GUI with real-time visualization
  - Batch processing capabilities
  - Configurable analysis parameters
  - Multiple visualization modes

- **Output and Export**
  - CSV data export
  - Detailed analysis reports
  - Configurable output formats
  - Batch processing results

## Installation

1. Ensure you have Rust installed (version 1.70 or later recommended). If not, install it from [rustup.rs](https://rustup.rs/)
2. Clone this repository:
   ```bash
   git clone https://github.com/thornbach/leafcomplexr.git
   cd leafcomplexr
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

### Basic Usage

Run the program with an input image:
```bash
cargo run --release -- --input "path/to/your/image.png"
```

### Interactive GUI Mode

Launch the interactive GUI:
```bash
cargo run --release -- --input "path/to/your/image.png" --gui
```

### Batch Processing

Process multiple images:
```bash
cargo run --release -- --input "path/to/directory" --batch
```

### Command Line Options

- `--input`: Path to input image or directory (required)
- `--gui`: Launch the interactive GUI
- `--batch`: Enable batch processing mode
- `--config`: Path to configuration file
- `--output`: Specify output directory
- `--kernel-size`: Set kernel size for morphological operations (default: 5)
- `--resize`: Resize input images (format: WIDTHxHEIGHT)

### GUI Controls

- **View Controls**
  - `T`: Toggle transparency view
  - `C`: Toggle CLR regions view
  - `P`: Toggle point analysis view
  - `S`: Toggle spectral analysis view
  - `Esc`: Exit program

- **Analysis Controls**
  - Use sliders to adjust kernel size and other parameters
  - Click on contour points for detailed analysis
  - Use mouse wheel to zoom
  - Drag to pan the view

## Project Structure

- `src/main.rs`: Program entry point and CLI handling
- `src/pipeline.rs`: Main processing pipeline
- `src/thornfiddle.rs`: Core analysis algorithms
- `src/config.rs`: Configuration management
- `src/output.rs`: Results export and formatting
- `src/point_analysis.rs`: Point-based feature analysis
- `src/shape_analysis.rs`: Shape and contour analysis
- `src/feature_extraction.rs`: Feature calculation
- `src/morphology.rs`: Morphological operations
- `src/path_algorithms.rs`: Path generation and analysis
- `src/image_utils.rs`: Image processing utilities
- `src/gui/`: GUI implementation
- `src/errors.rs`: Error handling

## Configuration

The program can be configured using a TOML configuration file. See the `config.rs` file for available options.

## Dependencies

- Core image processing: `image`, `imageproc`
- Configuration: `serde`, `toml`
- CLI: `clap`
- Data export: `csv`
- Error handling: `thiserror`, `anyhow`
- Parallel processing: `rayon`
- Geometry: `nalgebra`, `bresenham`
- GUI: `minifb`
- Spectral analysis: `rustfft`

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## Citation

If you use this software in your research, please cite:
```
@software{leafcomplexr,
  author = {Tobias Müller},
  title = {LeafComplexR},
  year = {2024},
  url = {https://github.com/thornbach/leafcomplexr}
}
```
