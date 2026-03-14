use base64::Engine;
use image::DynamicImage;
use std::fs;
use std::io::Cursor;
use std::path::Path;

const THUMB_MAX: u32 = 512;

/// Save image as JPEG with given quality.
pub fn save_jpeg(img: &DynamicImage, path: &Path, quality: u8) -> Result<(), String> {
    let rgb = img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    rgb.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode JPEG: {e}"))?;
    fs::write(path, buf.into_inner()).map_err(|e| format!("failed to write file: {e}"))?;
    Ok(())
}

/// Create a thumbnail (max 512px on longest side) and save as JPEG.
pub fn make_thumbnail(img: &DynamicImage, save_path: &Path) -> Result<(), String> {
    let thumb = img.resize(THUMB_MAX, THUMB_MAX, image::imageops::FilterType::Lanczos3);
    save_jpeg(&thumb, save_path, 85)
}

/// Read an image file and return its base64 data URL.
pub fn image_to_base64_url(path: &str) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("failed to read image: {e}"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:image/jpeg;base64,{b64}"))
}

/// Load a DynamicImage from raw bytes.
pub fn load_image_from_bytes(data: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(data).map_err(|e| format!("failed to load image: {e}"))
}

/// Resize image back to original dimensions if different.
pub fn resize_to_original(img: &DynamicImage, original_size: (u32, u32)) -> DynamicImage {
    let (ow, oh) = original_size;
    if img.width() == ow && img.height() == oh {
        return img.clone();
    }
    img.resize_exact(ow, oh, image::imageops::FilterType::Lanczos3)
}

/// Encode a DynamicImage to JPEG bytes for API submission.
pub fn image_to_jpeg_bytes(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let rgb = img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    rgb.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode JPEG: {e}"))?;
    Ok(buf.into_inner())
}

/// Encode image bytes to base64 string (no data URL prefix).
pub fn bytes_to_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Decode base64 string to bytes.
pub fn base64_to_bytes(b64: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode error: {e}"))
}

/// Resize image so longest side is at most `max_dim` pixels. No-op if already smaller.
pub fn resize_max_dimension(img: &DynamicImage, max_dim: u32) -> DynamicImage {
    if img.width() <= max_dim && img.height() <= max_dim {
        return img.clone();
    }
    img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
}

/// Load image from file path.
pub fn load_image_from_path(path: &str) -> Result<DynamicImage, String> {
    image::open(path)
        .map(|img| img.into())
        .map_err(|e| format!("failed to open image {path}: {e}"))
}
