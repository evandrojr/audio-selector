use image::{GenericImageView, ImageBuffer, imageops::FilterType, Pixel};

fn main() {
    let input_path = "assets/icon.jpeg";
    let output_path = "ui/assets/icon.png";

    let img = image::open(input_path).expect("Failed to open input image");
    let (orig_width, orig_height) = img.dimensions();
    
    // Determine the square size (use the shortest side to avoid distortion)
    let size = std::cmp::min(orig_width, orig_height);
    
    // Calculate crop offset to center the square
    let x_offset = (orig_width - size) / 2;
    let y_offset = (orig_height - size) / 2;
    
    // Crop the image to a perfect square
    let square_img = img.view(x_offset, y_offset, size, size).to_image();
    
    // Assume top-left corner is the background color to remove
    let bg_color = square_img.get_pixel(0, 0);
    let mut out_img = ImageBuffer::new(size, size);
    let threshold = 35; // Tolerance

    for x in 0..size {
        for y in 0..size {
            let p = square_img.get_pixel(x, y);
            let mut rgba = p.to_rgba();
            
            let r_diff = (rgba[0] as i32 - bg_color[0] as i32).abs();
            let g_diff = (rgba[1] as i32 - bg_color[1] as i32).abs();
            let b_diff = (rgba[2] as i32 - bg_color[2] as i32).abs();
            
            // If close to background color, make transparent
            if r_diff < threshold && g_diff < threshold && b_diff < threshold {
                rgba[3] = 0;
            }
            out_img.put_pixel(x, y, rgba);
        }
    }

    // Resize to standard icon size
    let final_img = image::imageops::resize(&out_img, 256, 256, FilterType::Lanczos3);
    final_img.save(output_path).expect("Failed to save final PNG");
    println!("Successfully cropped to square and saved to {}", output_path);
}
