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

// ---------------------------------------------------------------------------
// Layer compositing
// ---------------------------------------------------------------------------

/// One layer's render inputs for `composite_layers`.
pub struct LayerInput<'a> {
    pub image: &'a DynamicImage,
    /// 0.0..=1.0, multiplied into the layer's own alpha channel.
    pub opacity: f32,
    /// "normal" | "multiply" | "screen" | "overlay" (unknown → normal)
    pub blend_mode: &'a str,
    pub visible: bool,
}

/// Separable blend function on normalized channels (0..1).
fn blend_channel(mode: &str, dst: f32, src: f32) -> f32 {
    match mode {
        "multiply" => dst * src,
        "screen" => 1.0 - (1.0 - dst) * (1.0 - src),
        "overlay" => {
            if dst <= 0.5 {
                2.0 * dst * src
            } else {
                1.0 - 2.0 * (1.0 - dst) * (1.0 - src)
            }
        }
        _ => src, // normal
    }
}

/// Flatten a bottom-to-top layer stack into a single image.
///
/// Standard Porter-Duff "over" with W3C separable blend modes: for each pixel,
/// with dst = accumulated canvas and src = the layer (alpha pre-multiplied by
/// layer opacity),
///   out_a = a_s + a_d·(1−a_s)
///   out_c = ( a_s·((1−a_d)·c_s + a_d·B(c_d,c_s)) + a_d·c_d·(1−a_s) ) / out_a
/// A stack of [opaque base, masked edit layer] with normal blend at opacity 1
/// reproduces `composite_with_mask` exactly.
///
/// The canvas takes the first layer's dimensions; other layers are resized to
/// match if they differ (defensive — stacks are stored at a uniform size).
pub fn composite_layers(layers: &[LayerInput]) -> Result<DynamicImage, String> {
    let first = layers.first().ok_or("composite_layers: empty layer stack")?;
    let (w, h) = (first.image.width(), first.image.height());

    // Accumulator in normalized straight-alpha RGBA
    let mut acc = vec![[0.0f32; 4]; (w * h) as usize];

    for layer in layers {
        if !layer.visible || layer.opacity <= 0.0 {
            continue;
        }
        let opacity = layer.opacity.clamp(0.0, 1.0);
        let resized;
        let img = if layer.image.width() != w || layer.image.height() != h {
            resized = layer
                .image
                .resize_exact(w, h, image::imageops::FilterType::Lanczos3);
            &resized
        } else {
            layer.image
        };
        let src_rgba = img.to_rgba8();

        for (i, px) in src_rgba.pixels().enumerate() {
            let a_s = px[3] as f32 / 255.0 * opacity;
            if a_s <= 0.0 {
                continue;
            }
            let dst = &mut acc[i];
            let a_d = dst[3];
            let out_a = a_s + a_d * (1.0 - a_s);
            if out_a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                let c_s = px[c] as f32 / 255.0;
                let c_d = dst[c];
                let mixed = (1.0 - a_d) * c_s + a_d * blend_channel(layer.blend_mode, c_d, c_s);
                dst[c] = (a_s * mixed + a_d * c_d * (1.0 - a_s)) / out_a;
            }
            dst[3] = out_a;
        }
    }

    let mut output = RgbaImage::new(w, h);
    for (i, px) in acc.iter().enumerate() {
        let x = i as u32 % w;
        let y = i as u32 / w;
        output.put_pixel(
            x,
            y,
            Rgba([
                (px[0] * 255.0).round().clamp(0.0, 255.0) as u8,
                (px[1] * 255.0).round().clamp(0.0, 255.0) as u8,
                (px[2] * 255.0).round().clamp(0.0, 255.0) as u8,
                (px[3] * 255.0).round().clamp(0.0, 255.0) as u8,
            ]),
        );
    }
    Ok(DynamicImage::ImageRgba8(output))
}

/// Extract an AI edit result into a standalone edit layer.
///
/// Layer alpha is the inverse of the protection mask (mask alpha=255 = protect
/// → layer alpha 0; mask alpha=0 = edit region → layer alpha 255; partial
/// values map linearly, preserving feathered/anti-aliased selection edges).
/// Compositing [base, this layer] therefore equals `composite_with_mask`.
pub fn extract_edit_layer(result: &DynamicImage, mask: &DynamicImage) -> DynamicImage {
    let (w, h) = (result.width(), result.height());
    let mask_resized = if mask.width() != w || mask.height() != h {
        mask.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
    } else {
        mask.clone()
    };
    let result_rgba = result.to_rgba8();
    let mask_rgba = mask_resized.to_rgba8();

    let mut output = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let px = result_rgba.get_pixel(x, y);
            let mask_a = mask_rgba.get_pixel(x, y)[3];
            output.put_pixel(x, y, Rgba([px[0], px[1], px[2], 255 - mask_a]));
        }
    }
    DynamicImage::ImageRgba8(output)
}
