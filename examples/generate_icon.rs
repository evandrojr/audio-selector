use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_filled_rect_mut};
use imageproc::rect::Rect;

fn main() {
    let size = 256;
    let mut img = RgbaImage::new(size, size);
    
    // Background: Modern Vibrant Gradient-ish
    let dark_blue = Rgba([20, 20, 40, 255]);
    let mid_blue = Rgba([40, 80, 200, 255]);
    let white = Rgba([255, 255, 255, 255]);
    
    // Smooth circle background
    for x in 0..size {
        for y in 0..size {
            let dx = x as i32 - 128;
            let dy = y as i32 - 128;
            if dx*dx + dy*dy < 120*120 {
                img.put_pixel(x, y, mid_blue);
            } else {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0])); // Transparent corners
            }
        }
    }

    let center_x = 128;
    let center_y = 128;

    // Stylized Headphones
    // Headband
    for r in 90..100 {
        imageproc::drawing::draw_hollow_circle_mut(&mut img, (center_x, center_y + 10), r, white);
    }
    // Cut bottom of headband
    draw_filled_rect_mut(&mut img, Rect::at(0, center_y + 15).of_size(256, 128), mid_blue);
    
    // Re-fill transparent corners after rect draw
    for x in 0..size {
        for y in 0..size {
            let dx = x as i32 - 128;
            let dy = y as i32 - 128;
            if dx*dx + dy*dy >= 120*120 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    // Ear pads
    let pad_w = 40;
    let pad_h = 70;
    draw_filled_rect_mut(&mut img, Rect::at(center_x - 90, center_y - 10).of_size(pad_w, pad_h), white);
    draw_filled_rect_mut(&mut img, Rect::at(center_x + 50, center_y - 10).of_size(pad_w, pad_h), white);

    // Modern Play Triangle in center
    draw_filled_circle_mut(&mut img, (center_x, center_y), 30, white);
    draw_filled_circle_mut(&mut img, (center_x, center_y), 25, mid_blue);
    
    img.save("ui/assets/icon.png").expect("Failed to save icon");
}
