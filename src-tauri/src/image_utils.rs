use base64::Engine;
use image::{DynamicImage, Rgba, RgbaImage};
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

/// Save image as PNG (lossless). Used for stored artifacts where we want
/// to avoid generation loss across multi-step agent edits.
pub fn save_png(img: &DynamicImage, path: &Path) -> Result<(), String> {
    let bytes = image_to_png_bytes(img)?;
    fs::write(path, bytes).map_err(|e| format!("failed to write file: {e}"))?;
    Ok(())
}

/// Encode a DynamicImage to PNG bytes (lossless).
pub fn image_to_png_bytes(img: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf.into_inner())
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
    let mut img =
        image::load_from_memory(data).map_err(|e| format!("failed to load image: {e}"))?;

    // Apply EXIF orientation if present
    if let Some(orientation) = read_exif_orientation(data) {
        img = match orientation {
            2 => img.fliph(),
            3 => img.rotate180(),
            4 => img.flipv(),
            5 => img.rotate90().fliph(),
            6 => img.rotate90(),
            7 => img.rotate270().fliph(),
            8 => img.rotate270(),
            _ => img,
        };
    }

    Ok(img)
}

fn read_exif_orientation(data: &[u8]) -> Option<u32> {
    let exif = exif::Reader::new()
        .read_from_container(&mut std::io::Cursor::new(data))
        .ok()?;
    let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    field.value.get_uint(0)
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

/// Composite API result onto original using mask.
/// Mask convention: alpha=255 (opaque white) = protect (use original),
/// alpha=0 (transparent) = edit (use result). Intermediate values blend linearly.
/// All three images must have the same dimensions.
pub fn composite_with_mask(
    original: &DynamicImage,
    result: &DynamicImage,
    mask: &DynamicImage,
) -> DynamicImage {
    let (w, h) = (original.width(), original.height());
    let orig_rgba = original.to_rgba8();
    let result_rgba = result.to_rgba8();
    let mask_rgba = mask.to_rgba8();

    let mut output = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let mask_a = mask_rgba.get_pixel(x, y)[3] as f32 / 255.0;
            let orig_px = orig_rgba.get_pixel(x, y);
            let res_px = result_rgba.get_pixel(x, y);

            let blended = Rgba([
                (orig_px[0] as f32 * mask_a + res_px[0] as f32 * (1.0 - mask_a)) as u8,
                (orig_px[1] as f32 * mask_a + res_px[1] as f32 * (1.0 - mask_a)) as u8,
                (orig_px[2] as f32 * mask_a + res_px[2] as f32 * (1.0 - mask_a)) as u8,
                255,
            ]);
            output.put_pixel(x, y, blended);
        }
    }

    DynamicImage::ImageRgba8(output)
}

/// Generate a mask overlay image for Gemini: original image with red semi-transparent
/// tint on edit regions (where mask is transparent). Returns base64-encoded PNG.
pub fn generate_mask_overlay(image_b64: &str, mask_b64: &str) -> Result<String, String> {
    let img_bytes = base64_to_bytes(image_b64)?;
    let img = load_image_from_bytes(&img_bytes)?;
    let mask_bytes = base64_to_bytes(mask_b64)?;
    let mask = load_image_from_bytes(&mask_bytes)?;

    let (w, h) = (img.width(), img.height());
    let mask_resized = if mask.width() != w || mask.height() != h {
        mask.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
    } else {
        mask
    };

    let img_rgba = img.to_rgba8();
    let mask_rgba = mask_resized.to_rgba8();
    let mut output = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let src = img_rgba.get_pixel(x, y);
            let mask_a = mask_rgba.get_pixel(x, y)[3] as f32 / 255.0;
            // Where mask is transparent (edit region), overlay red tint
            let red_blend = 1.0 - mask_a; // 0 at protected, 1 at edit
            let tint_strength = 0.45;
            let r = (src[0] as f32 * (1.0 - red_blend * tint_strength)
                + 255.0 * red_blend * tint_strength) as u8;
            let g = (src[1] as f32 * (1.0 - red_blend * tint_strength * 0.8)) as u8;
            let b = (src[2] as f32 * (1.0 - red_blend * tint_strength * 0.8)) as u8;
            output.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    let overlay_img = DynamicImage::ImageRgba8(output);
    let png_bytes = image_to_png_bytes(&overlay_img)?;
    Ok(bytes_to_base64(&png_bytes))
}
