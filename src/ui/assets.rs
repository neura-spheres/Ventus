use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;

static LOGO_PNG: &[u8] = include_bytes!("../../public/ventus.png");
static LOGO_TEXT_WHITE: &[u8] = include_bytes!("../../public/textlogoWhite.png");
static LOGO_TEXT_BLACK: &[u8] = include_bytes!("../../public/textlogoBlack.png");

pub fn logo_data_url() -> String {
    let img = image::load_from_memory(LOGO_PNG)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(64, 64));
    let resized = img.resize(64, 64, image::imageops::FilterType::Lanczos3);
    let mut buf = Vec::new();
    let _ = resized.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    format!("data:image/png;base64,{}", STANDARD.encode(&buf))
}

pub fn logo_text_white_data_url() -> String {
    wordmark_data_url(LOGO_TEXT_WHITE)
}

pub fn logo_text_black_data_url() -> String {
    wordmark_data_url(LOGO_TEXT_BLACK)
}

fn wordmark_data_url(bytes: &[u8]) -> String {
    let Ok(img) = image::load_from_memory(bytes) else {
        return String::new();
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0u32, 0u32);
    for (x, y, px) in rgba.enumerate_pixels() {
        if px[3] <= 16 {
            continue;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    if max_x < min_x || max_y < min_y {
        return String::new();
    }
    let pad = 6u32;
    let cx = min_x.saturating_sub(pad);
    let cy = min_y.saturating_sub(pad);
    let cw = (max_x - min_x + 1 + pad * 2).min(w - cx);
    let ch = (max_y - min_y + 1 + pad * 2).min(h - cy);
    let trimmed = image::DynamicImage::ImageRgba8(rgba).crop_imm(cx, cy, cw, ch);
    let resized = trimmed.resize(640, 96, image::imageops::FilterType::Lanczos3);
    let mut buf = Vec::new();
    let _ = resized.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    format!("data:image/png;base64,{}", STANDARD.encode(&buf))
}
