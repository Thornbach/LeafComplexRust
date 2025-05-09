use image::{ImageBuffer, Rgba, RgbaImage};

/// Constants
pub const ALPHA_THRESHOLD: u8 = 128; // Alpha value above which a pixel is considered non-transparent

/// Resize an image to the specified dimensions
pub fn resize_image(
    image: &RgbaImage,
    dimensions: [u32; 2],
) -> RgbaImage {
    let (width, height) = (dimensions[0], dimensions[1]);
    image::imageops::resize(
        image,
        width,
        height,
        image::imageops::FilterType::Triangle,
    )
}

/// Check if a pixel is transparent (alpha below threshold)
#[inline]
pub fn is_transparent(pixel: &Rgba<u8>) -> bool {
    pixel[3] < ALPHA_THRESHOLD
}

/// Check if a pixel is non-transparent (alpha >= threshold)
#[inline]
pub fn is_non_transparent(pixel: &Rgba<u8>) -> bool {
    pixel[3] >= ALPHA_THRESHOLD
}

/// Check if a pixel has the specified RGB color (ignoring alpha)
#[inline]
pub fn has_rgb_color(pixel: &Rgba<u8>, color: [u8; 3]) -> bool {
    pixel[0] == color[0] && pixel[1] == color[1] && pixel[2] == color[2]
}

/// Create a mask image from alpha channel (255 for non-transparent, 0 for transparent)
pub fn create_alpha_mask(image: &RgbaImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = image.dimensions();
    let mut mask = ImageBuffer::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let mask_value = if is_non_transparent(pixel) { 255 } else { 0 };
            mask.put_pixel(x, y, Rgba([mask_value, mask_value, mask_value, 255]));
        }
    }
    
    mask
}

/// Convert coordinates between image systems if needed
/// In most image processing systems, (0,0) is top-left and y increases downward
#[inline]
pub fn convert_coordinates(x: u32, y: u32, _height: u32) -> (u32, u32) {
    // This function could be modified if a different coordinate system is needed
    (x, y)
}

/// Generate a circular kernel of a given size (diameter)
pub fn create_circular_kernel(diameter: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let radius = (diameter as f32) / 2.0;
    let center = radius;
    
    let mut kernel = ImageBuffer::new(diameter, diameter);
    
    for y in 0..diameter {
        for x in 0..diameter {
            let dx = (x as f32) - center;
            let dy = (y as f32) - center;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance <= radius {
                kernel.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            } else {
                kernel.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    
    kernel
}

/// Check if a point is inside the image bounds
#[inline]
pub fn in_bounds(x: i32, y: i32, width: u32, height: u32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height
}

/// Create a debug image with specified points marked in color
pub fn create_debug_image(
    image: &RgbaImage,
    points: &[(u32, u32)],
    color: [u8; 3],
    point_size: u32,
) -> RgbaImage {
    let mut debug_image = image.clone();
    let (width, height) = debug_image.dimensions();
    
    for &(x, y) in points {
        let radius = point_size / 2;
        for dy in 0..point_size {
            for dx in 0..point_size {
                let px = x.saturating_sub(radius).saturating_add(dx);
                let py = y.saturating_sub(radius).saturating_add(dy);
                
                if px < width && py < height {
                    debug_image.put_pixel(px, py, Rgba([color[0], color[1], color[2], 255]));
                }
            }
        }
    }
    
    debug_image
}