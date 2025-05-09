# LeafComplexR

LeafComplexR is a high-performance Rust application for analyzing leaf morphology from PNG images with transparency. The program extracts detailed geometric and path-based features using image manipulation, point identification, straight-line path analysis, and complex Golden Spiral path generation algorithms.

## Features

- Interactive GUI for leaf analysis
- Golden spiral path generation
- Contour tracing and analysis
- CLR (Complex Leaf Region) calculation
- Support for both LEC (Leaf Edge Complexity) and LMC (Leaf Margin Complexity) analysis

## Installation

1. Make sure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs/)
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

Run the program with an input image:
```bash
cargo run --release -- --input path/to/your/image.png --gui
```

### Command Line Options

- `--input`: Path to the input image (required)
- `--gui`: Launch the interactive GUI
- `--kernel-size`: Set the kernel size for morphological operations (default: 5)
- `--resize`: Resize the input image (format: WIDTHxHEIGHT)

### GUI Controls

- Click on contour points to analyze them
- Use the slider to adjust kernel size
- Press 'T' to toggle transparency view
- Press 'C' to toggle CLR regions view
- Press 'Esc' to exit

## Project Structure

- `src/gui.rs`: GUI implementation
- `src/path_algorithms.rs`: Golden spiral and path calculation algorithms
- `src/feature_extraction.rs`: Feature calculation and analysis
- `src/morphology.rs`: Morphological operations
- `src/image_utils.rs`: Image processing utilities

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.