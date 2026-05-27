use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;

static LOGO_PNG: &[u8] = include_bytes!("../../public/ventus.png");

pub fn logo_data_url() -> String {
    let img = image::load_from_memory(LOGO_PNG)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(64, 64));
    let resized = img.resize(64, 64, image::imageops::FilterType::Lanczos3);
    let mut buf = Vec::new();
    let _ = resized.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    format!("data:image/png;base64,{}", STANDARD.encode(&buf))
}
