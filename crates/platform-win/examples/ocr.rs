//! Reads the text in an image file with Windows OCR: `cargo run -p
//! spotlight-platform-win --example ocr -- picture.png`.

fn main() {
    let path = std::env::args().nth(1).expect("an image file");
    let image = image::open(&path).expect("a readable image").into_rgba8();
    match spotlight_platform_win::ocr::read(&image) {
        Ok(text) => println!("{text}"),
        Err(err) => eprintln!("{err}"),
    }
}
