//! Unit tests for the layer compositor (no network required).
//!
//! The critical invariant: flattening [opaque base, masked edit layer] with
//! normal blend at opacity 1 must reproduce `composite_with_mask` — that is
//! what guarantees a node's layer stack and its stored flat image agree.

use coskit::image_utils::{
    composite_layers, composite_with_mask, extract_edit_layer, LayerInput,
};
use image::{DynamicImage, Rgba, RgbaImage};

/// Deterministic synthetic image with per-pixel varying channels.
fn pattern_image(w: u32, h: u32, seed: u8) -> DynamicImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(
                x,
                y,
                Rgba([
                    ((x * 7 + y * 3) as u8).wrapping_add(seed),
                    ((x * 13 + y * 5) as u8).wrapping_mul(2).wrapping_add(seed),
                    ((x + y * 11) as u8).wrapping_add(seed.wrapping_mul(3)),
                    255,
                ]),
            );
        }
    }
    DynamicImage::ImageRgba8(img)
}

/// Mask with a soft-edged transparent rectangle (edit region), mimicking a
/// feathered frontend selection: alpha 0 inside, 255 outside, gradient ring.
fn soft_mask(w: u32, h: u32) -> DynamicImage {
    let mut mask = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    let (x0, x1, y0, y1) = (w / 4, w * 3 / 4, h / 4, h * 3 / 4);
    for y in y0..y1 {
        for x in x0..x1 {
            mask.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }
    // Gradient ring row above the rectangle: partial alpha values
    for x in x0..x1 {
        let a = ((x - x0) * 255 / (x1 - x0).max(1)) as u8;
        mask.put_pixel(x, y0 - 1, Rgba([255, 255, 255, a]));
    }
    DynamicImage::ImageRgba8(mask)
}

fn max_channel_diff(a: &DynamicImage, b: &DynamicImage) -> u8 {
    let ra = a.to_rgba8();
    let rb = b.to_rgba8();
    let mut max = 0u8;
    for (pa, pb) in ra.pixels().zip(rb.pixels()) {
        for c in 0..3 {
            max = max.max(pa[c].abs_diff(pb[c]));
        }
    }
    max
}

#[test]
fn base_plus_edit_layer_equals_composite_with_mask() {
    let base = pattern_image(64, 48, 11);
    let result = pattern_image(64, 48, 199);
    let mask = soft_mask(64, 48);

    let expected = composite_with_mask(&base, &result, &mask);

    let edit_layer = extract_edit_layer(&result, &mask);
    let flat = composite_layers(&[
        LayerInput { image: &base, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &edit_layer, opacity: 1.0, blend_mode: "normal", visible: true },
    ])
    .expect("composite");

    // f32 round-trip may differ by 1 on partial-alpha edge pixels
    assert!(
        max_channel_diff(&expected, &flat) <= 1,
        "layer flatten must match composite_with_mask (diff {})",
        max_channel_diff(&expected, &flat)
    );
}

#[test]
fn hidden_and_zero_opacity_layers_are_noops() {
    let base = pattern_image(32, 32, 5);
    let overlay = pattern_image(32, 32, 77);

    let flat = composite_layers(&[
        LayerInput { image: &base, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &overlay, opacity: 1.0, blend_mode: "normal", visible: false },
        LayerInput { image: &overlay, opacity: 0.0, blend_mode: "normal", visible: true },
    ])
    .expect("composite");

    assert_eq!(max_channel_diff(&base, &flat), 0, "hidden layers must not affect output");
}

#[test]
fn half_opacity_blends_midway() {
    let black = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255])));
    let white = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, Rgba([255, 255, 255, 255])));

    let flat = composite_layers(&[
        LayerInput { image: &black, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &white, opacity: 0.5, blend_mode: "normal", visible: true },
    ])
    .expect("composite");

    let px = flat.to_rgba8().get_pixel(4, 4).0;
    for c in 0..3 {
        assert!((px[c] as i32 - 128).abs() <= 1, "expected ~128, got {}", px[c]);
    }
    assert_eq!(px[3], 255);
}

#[test]
fn blend_modes_behave() {
    let gray = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([100, 100, 100, 255])));
    let half = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([128, 128, 128, 255])));

    // multiply: 100/255 * 128/255 * 255 ≈ 50
    let m = composite_layers(&[
        LayerInput { image: &gray, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &half, opacity: 1.0, blend_mode: "multiply", visible: true },
    ])
    .unwrap();
    let px = m.to_rgba8().get_pixel(0, 0).0;
    assert!((px[0] as i32 - 50).abs() <= 1, "multiply expected ~50, got {}", px[0]);

    // screen: 255 - (155*127)/255 ≈ 178
    let s = composite_layers(&[
        LayerInput { image: &gray, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &half, opacity: 1.0, blend_mode: "screen", visible: true },
    ])
    .unwrap();
    let px = s.to_rgba8().get_pixel(0, 0).0;
    assert!((px[0] as i32 - 178).abs() <= 1, "screen expected ~178, got {}", px[0]);

    // overlay on dst<=0.5: 2*dst*src → 2*(100/255)*(128/255)*255 ≈ 100
    let o = composite_layers(&[
        LayerInput { image: &gray, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &half, opacity: 1.0, blend_mode: "overlay", visible: true },
    ])
    .unwrap();
    let px = o.to_rgba8().get_pixel(0, 0).0;
    assert!((px[0] as i32 - 100).abs() <= 2, "overlay expected ~100, got {}", px[0]);
}

#[test]
fn transparent_regions_of_edit_layer_reveal_base_after_reorder() {
    // A masked edit layer sandwiched between two opaque layers: moving it to
    // the top must change the output only inside its opaque (edit) region.
    let base = pattern_image(32, 32, 1);
    let full = pattern_image(32, 32, 120);
    let result = pattern_image(32, 32, 230);
    let mask = soft_mask(32, 32);
    let edit = extract_edit_layer(&result, &mask);

    let flat = composite_layers(&[
        LayerInput { image: &base, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &full, opacity: 1.0, blend_mode: "normal", visible: true },
        LayerInput { image: &edit, opacity: 1.0, blend_mode: "normal", visible: true },
    ])
    .unwrap();

    let flat_rgba = flat.to_rgba8();
    let full_rgba = full.to_rgba8();
    let result_rgba = result.to_rgba8();
    let mask_rgba = mask.to_rgba8();

    for y in 0..32u32 {
        for x in 0..32u32 {
            let a = mask_rgba.get_pixel(x, y)[3];
            let f = flat_rgba.get_pixel(x, y);
            if a == 255 {
                // protected: middle opaque layer shows through
                assert_eq!(f, full_rgba.get_pixel(x, y), "protected px ({x},{y})");
            } else if a == 0 {
                // edit region: top layer wins
                assert_eq!(f, result_rgba.get_pixel(x, y), "edit px ({x},{y})");
            }
        }
    }
}
