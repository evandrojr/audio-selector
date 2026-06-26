use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_circle_mut};
use imageproc::rect::Rect;

fn main() {
    let size = 256;
    let mut img = RgbaImage::new(size, size);
    
    // Background: Deep Blue Rounded Rect-ish
    let bg_color = Rgba([30, 58, 138, 255]);
    let accent_color = Rgba([255, 255, 255, 255]);
    
    // Fill background with a nice gradient or solid color
    for x in 0..size {
        for y in 0..size {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Draw stylized headphones
    let center_x = (size / 2) as i32;
    let center_y = (size / 2) as i32;
    
    // 1. Arc (Headband)
    for i in 0..5 { // Thickness
        draw_hollow_circle_mut(&mut img, (center_x, center_y + 20), 80 - i, accent_color);
    }
    
    // Clear the bottom half of the circle to make it an arc
    draw_filled_rect_mut(&mut img, Rect::at(0, center_y + 25).of_size(size, size / 2), bg_color);

    // 2. Ear cups (Left and Right)
    let cup_width = 45;
    let cup_height = 70;
    let corner_radius = 15;
    
    // Left Cup
    draw_filled_rect_mut(&mut img, Rect::at(center_x - 95, center_y - 10).of_size(cup_width as u32, cup_height as u32), accent_color);
    // Right Cup
    draw_filled_rect_mut(&mut img, Rect::at(center_x + 50, center_y - 10).of_size(cup_width as u32, cup_height as u32), accent_color);

    // 3. Audio Waves / Signal (Minimalist)
    draw_filled_circle_mut(&mut img, (center_x, center_y + 30), 8, accent_color);
    draw_hollow_circle_mut(&mut img, (center_x, center_y + 30), 20, accent_color);
    draw_hollow_circle_mut(&mut img, (center_x, center_y + 30), 35, accent_color);

    img.save("ui/assets/icon.png").expect("Failed to save icon");
    println!("Custom icon generated at ui/assets/icon.png");
}
