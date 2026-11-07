//! Draws the region picker over a made-up screen into a PNG, to check its
//! design without covering the real screen: `cargo run -p
//! spotlight-platform-win --example region_preview -- out.png [hover]`.

use image::{Rgba, RgbaImage};
use windows::Win32::Foundation::RECT;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("where to save the PNG");
    let hover = args.next().and_then(|arg| arg.parse().ok());
    // A light page with a card on it.
    let mut screen = RgbaImage::from_fn(1100, 680, |_, y| {
        let shade = 236 - (y / 40) as u8;
        Rgba([shade, shade, (shade as u16 + 6).min(255) as u8, 255])
    });
    for y in 150..380 {
        for x in 260..760 {
            screen.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    for y in 200..215 {
        for x in 300..640 {
            screen.put_pixel(x, y, Rgba([40, 40, 48, 255]));
        }
    }
    let selection = RECT {
        left: 230,
        top: 120,
        right: 790,
        bottom: 410,
    };
    let image = spotlight_platform_win::region::preview(screen, selection, hover).unwrap();
    image.save(&out).unwrap();
}
