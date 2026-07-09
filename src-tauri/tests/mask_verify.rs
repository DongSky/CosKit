//! End-to-end mask verification (requires configured API credentials).
//!
//! Simulates the exact frontend → backend mask flow:
//!   1. Load a test image, downscale like `run_edit_pipeline` (≤2048px).
//!   2. Build a mask the way the frontend MaskEditor exports it:
//!      working size ≤1920 floored to /16 (here 1344×768 vs image 1344×776 —
//!      deliberately different so the OpenAI same-dimensions resize fix is
//!      exercised), white opaque = protect, transparent rect = edit region.
//!   3. Call `call_image_generation` with the mask (native mask on OpenAI,
//!      overlay+prompt strategy on Gemini).
//!   4. Composite the result with `composite_with_mask` and assert that every
//!      fully-protected pixel is bit-identical to the original.
//!
//! Run manually (network + credentials required):
//!   cargo test --test mask_verify -- --ignored --nocapture

use coskit::gemini_client::{self, GeminiClients};
use coskit::image_utils;
use image::{DynamicImage, Rgba, RgbaImage};
use std::path::Path;

fn out_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_output")
}

/// Mimic the frontend MaskEditor working size: ≤1920 on the long edge,
/// both edges floored to a multiple of 16.
fn working_size(w: u32, h: u32) -> (u32, u32) {
    let max = 1920.0f64;
    let scale = (max / w.max(h) as f64).min(1.0);
    let fw = ((w as f64 * scale / 16.0).floor() * 16.0) as u32;
    let fh = ((h as f64 * scale / 16.0).floor() * 16.0) as u32;
    (fw, fh)
}

/// Ray-casting point-in-polygon (even-odd; identical to non-zero winding for
/// simple non-self-intersecting polygons, which is what the editor produces).
fn point_in_polygon(px: f32, py: f32, pts: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Mimic `_applyPolygon` in mask-editor.js: canvas `fill()` with
/// `destination-out` punches a transparent hole into the white mask, with
/// anti-aliased edges. Reproduced via 4x4 supersampling — interior alpha=0,
/// boundary pixels get partial alpha proportional to coverage.
fn punch_polygon_aa(mask: &mut RgbaImage, pts: &[(f32, f32)]) {
    let x0 = pts.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor().max(0.0) as u32;
    let x1 = (pts.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as u32).min(mask.width());
    let y0 = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor().max(0.0) as u32;
    let y1 = (pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as u32).min(mask.height());
    for y in y0..y1 {
        for x in x0..x1 {
            let mut hits = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    if point_in_polygon(px, py, pts) {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                let p = mask.get_pixel_mut(x, y);
                // destination-out: dst_alpha *= (1 - src_coverage)
                p[3] = (p[3] as f32 * (1.0 - hits as f32 / 16.0)).round() as u8;
            }
        }
    }
}

#[test]
#[ignore]
fn mask_end_to_end() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        // --- Init clients from settings.json / .env (same path as the app) ---
        GeminiClients::init().expect("init clients");
        let clients = gemini_client::get_clients().expect("get clients");
        eprintln!(
            "[mask_verify] image provider = {}, model = {}",
            clients.image_provider, clients.image_model
        );

        // --- Load source image ---
        let src_path = out_dir().join("source.jpeg");
        let original = image_utils::load_image_from_path(src_path.to_str().unwrap())
            .expect("load source image");
        let (ow, oh) = (original.width(), original.height());
        eprintln!("[mask_verify] source {}x{}", ow, oh);

        // Same downscale as run_edit_pipeline
        let api_img = image_utils::resize_max_dimension(&original, 2048);
        let img_b64 = image_utils::bytes_to_base64(
            &image_utils::image_to_png_bytes(&api_img).expect("encode api image"),
        );

        // --- Build the mask exactly like the frontend export ---
        // White opaque everywhere (protect), fully transparent rectangle over
        // the LEFT figure (edit region).
        let (mw, mh) = working_size(ow, oh);
        eprintln!("[mask_verify] mask working size {}x{}", mw, mh);
        let mut mask = RgbaImage::from_pixel(mw, mh, Rgba([255, 255, 255, 255]));
        // Left figure ≈ x 17%..40% of width, y 11%..100% of height
        let (rx0, rx1) = ((mw as f32 * 0.17) as u32, (mw as f32 * 0.40) as u32);
        let (ry0, ry1) = ((mh as f32 * 0.11) as u32, mh);
        for y in ry0..ry1 {
            for x in rx0..rx1 {
                mask.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        let mask_img = DynamicImage::ImageRgba8(mask);
        let mask_png = image_utils::image_to_png_bytes(&mask_img).expect("encode mask");
        std::fs::write(out_dir().join("mask.png"), &mask_png).expect("save mask");
        let mask_b64 = image_utils::bytes_to_base64(&mask_png);

        // --- Call the image model through the real mask path ---
        let prompt = "将选区内（画面左侧的人物）的头发颜色改为银白色。\
                      保持发型轮廓、服装、姿势和光照不变。\
                      画面右侧的人物和背景不得有任何改动。";
        eprintln!("[mask_verify] calling image API with mask...");
        let t0 = std::time::Instant::now();
        let result_bytes = gemini_client::call_image_generation(
            &img_b64,
            prompt,
            &[],
            0.3,
            Some((ow, oh)),
            Some(&mask_b64),
        )
        .await
        .expect("image generation with mask");
        eprintln!(
            "[mask_verify] API returned {} bytes in {:.1}s",
            result_bytes.len(),
            t0.elapsed().as_secs_f32()
        );
        std::fs::write(out_dir().join("raw_result.png"), &result_bytes)
            .expect("save raw result");

        // --- Composite exactly like run_edit_pipeline ---
        let api_result = image_utils::load_image_from_bytes(&result_bytes)
            .expect("decode api result");
        eprintln!(
            "[mask_verify] raw result {}x{}",
            api_result.width(),
            api_result.height()
        );
        let result_resized = image_utils::resize_to_original(&api_result, (ow, oh));
        let mask_resized = image_utils::resize_to_original(&mask_img, (ow, oh));
        let final_img =
            image_utils::composite_with_mask(&original, &result_resized, &mask_resized);
        image_utils::save_png(&final_img, &out_dir().join("final.png")).expect("save final");

        // --- Verification ---
        let orig_rgba = original.to_rgba8();
        let raw_rgba = result_resized.to_rgba8();
        let final_rgba = final_img.to_rgba8();
        let mask_rgba = mask_resized.to_rgba8();

        let mut max_diff_protected = 0u8; // final vs original where alpha==255
        let mut raw_changed_protected = 0u64; // raw API result changed protected pixels
        let mut protected_count = 0u64;
        let mut edited_changed = 0u64; // final changed inside edit region
        let mut edited_count = 0u64;

        for y in 0..oh {
            for x in 0..ow {
                let a = mask_rgba.get_pixel(x, y)[3];
                let o = orig_rgba.get_pixel(x, y);
                let f = final_rgba.get_pixel(x, y);
                let r = raw_rgba.get_pixel(x, y);
                let df = (0..3).map(|i| o[i].abs_diff(f[i])).max().unwrap();
                let dr = (0..3).map(|i| o[i].abs_diff(r[i])).max().unwrap();
                if a == 255 {
                    protected_count += 1;
                    max_diff_protected = max_diff_protected.max(df);
                    if dr > 12 {
                        raw_changed_protected += 1;
                    }
                } else if a == 0 {
                    edited_count += 1;
                    if df > 12 {
                        edited_changed += 1;
                    }
                }
            }
        }

        eprintln!("[mask_verify] ── 验证结果 ──");
        eprintln!(
            "[mask_verify] 保护区像素: {} | final vs original 最大差值: {} (必须为 0)",
            protected_count, max_diff_protected
        );
        eprintln!(
            "[mask_verify] 原始 API 结果在保护区改动的像素: {} ({:.1}%) ← 合成兜底修掉的部分",
            raw_changed_protected,
            raw_changed_protected as f64 / protected_count as f64 * 100.0
        );
        eprintln!(
            "[mask_verify] 选区内实际发生变化的像素: {} / {} ({:.1}%)",
            edited_changed,
            edited_count,
            edited_changed as f64 / edited_count.max(1) as f64 * 100.0
        );

        assert_eq!(
            max_diff_protected, 0,
            "保护区(mask alpha=255)像素必须与原图完全一致"
        );
        assert!(
            edited_changed > 0,
            "选区内应有实际编辑发生（API 未生效？）"
        );
        eprintln!("[mask_verify] ✅ PASS — 选区外零改动，选区内编辑生效");
    });
}

/// Polygon-mask variant: same E2E path, but the edit region is an irregular
/// 8-vertex polygon around the RIGHT figure, rasterized exactly like the
/// frontend polygon tool exports it (anti-aliased fill punched with
/// destination-out → partial alpha on the boundary ring). Verifies the
/// polygon geometry survives resize + compositing and that partial-alpha
/// edge pixels blend instead of tearing.
#[test]
#[ignore]
fn polygon_mask_end_to_end() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        GeminiClients::init().expect("init clients");
        let clients = gemini_client::get_clients().expect("get clients");
        eprintln!(
            "[poly_verify] image provider = {}, model = {}",
            clients.image_provider, clients.image_model
        );

        let src_path = out_dir().join("source.jpeg");
        let original = image_utils::load_image_from_path(src_path.to_str().unwrap())
            .expect("load source image");
        let (ow, oh) = (original.width(), original.height());

        let api_img = image_utils::resize_max_dimension(&original, 2048);
        let img_b64 = image_utils::bytes_to_base64(
            &image_utils::image_to_png_bytes(&api_img).expect("encode api image"),
        );

        // --- Polygon around the right figure, vertices as clicked in the
        // editor (fractions of the working canvas) ---
        let (mw, mh) = working_size(ow, oh);
        eprintln!("[poly_verify] mask working size {}x{}", mw, mh);
        let verts: Vec<(f32, f32)> = [
            (0.670, 0.140), // above the hair bow, left
            (0.795, 0.140), // above the hair bow, right
            (0.845, 0.440), // right sleeve
            (0.860, 0.730), // kimono tail
            (0.810, 0.965), // right foot
            (0.695, 0.968), // left foot
            (0.678, 0.640), // left leg edge
            (0.655, 0.360), // back / left shoulder
        ]
        .iter()
        .map(|(fx, fy)| (fx * mw as f32, fy * mh as f32))
        .collect();

        let mut mask = RgbaImage::from_pixel(mw, mh, Rgba([255, 255, 255, 255]));
        punch_polygon_aa(&mut mask, &verts);

        // Mask stats: prove we actually produced an AA polygon, not a rect
        let (mut n_edit, mut n_partial) = (0u64, 0u64);
        for p in mask.pixels() {
            match p[3] {
                0 => n_edit += 1,
                255 => {}
                _ => n_partial += 1,
            }
        }
        eprintln!(
            "[poly_verify] 多边形选区: {} px ({:.1}%), 抗锯齿边缘半透明像素: {}",
            n_edit,
            n_edit as f64 / (mw * mh) as f64 * 100.0,
            n_partial
        );
        assert!(n_partial > 0, "多边形斜边应产生抗锯齿半透明像素");

        let mask_img = DynamicImage::ImageRgba8(mask);
        let mask_png = image_utils::image_to_png_bytes(&mask_img).expect("encode mask");
        std::fs::write(out_dir().join("polygon_mask.png"), &mask_png).expect("save mask");
        let mask_b64 = image_utils::bytes_to_base64(&mask_png);

        let prompt = "将选区内（画面右侧的人物）的头发颜色改为粉色。\
                      保持发型轮廓、服装、姿势和光照不变。\
                      画面左侧的人物和背景不得有任何改动。";
        eprintln!("[poly_verify] calling image API with polygon mask...");
        let t0 = std::time::Instant::now();
        let result_bytes = gemini_client::call_image_generation(
            &img_b64,
            prompt,
            &[],
            0.3,
            Some((ow, oh)),
            Some(&mask_b64),
        )
        .await
        .expect("image generation with polygon mask");
        eprintln!(
            "[poly_verify] API returned {} bytes in {:.1}s",
            result_bytes.len(),
            t0.elapsed().as_secs_f32()
        );
        std::fs::write(out_dir().join("polygon_raw.png"), &result_bytes)
            .expect("save raw result");

        let api_result =
            image_utils::load_image_from_bytes(&result_bytes).expect("decode api result");
        eprintln!(
            "[poly_verify] raw result {}x{}",
            api_result.width(),
            api_result.height()
        );
        let result_resized = image_utils::resize_to_original(&api_result, (ow, oh));
        let mask_resized = image_utils::resize_to_original(&mask_img, (ow, oh));
        let final_img =
            image_utils::composite_with_mask(&original, &result_resized, &mask_resized);
        image_utils::save_png(&final_img, &out_dir().join("polygon_final.png"))
            .expect("save final");

        // --- Verification: alpha==255 protected must be bit-identical;
        // alpha==0 interior should show edits; partial-alpha ring must stay
        // between original and result (blend sanity, no tearing) ---
        let orig_rgba = original.to_rgba8();
        let raw_rgba = result_resized.to_rgba8();
        let final_rgba = final_img.to_rgba8();
        let mask_rgba = mask_resized.to_rgba8();

        let mut max_diff_protected = 0u8;
        let mut raw_changed_protected = 0u64;
        let mut protected_count = 0u64;
        let mut edited_changed = 0u64;
        let mut edited_count = 0u64;
        let mut partial_count = 0u64;
        let mut partial_blend_violation = 0u64;

        for y in 0..oh {
            for x in 0..ow {
                let a = mask_rgba.get_pixel(x, y)[3];
                let o = orig_rgba.get_pixel(x, y);
                let f = final_rgba.get_pixel(x, y);
                let r = raw_rgba.get_pixel(x, y);
                let df = (0..3).map(|i| o[i].abs_diff(f[i])).max().unwrap();
                let dr = (0..3).map(|i| o[i].abs_diff(r[i])).max().unwrap();
                match a {
                    255 => {
                        protected_count += 1;
                        max_diff_protected = max_diff_protected.max(df);
                        if dr > 12 {
                            raw_changed_protected += 1;
                        }
                    }
                    0 => {
                        edited_count += 1;
                        if df > 12 {
                            edited_changed += 1;
                        }
                    }
                    _ => {
                        // final must lie within [min(o,r)-1, max(o,r)+1] per channel
                        partial_count += 1;
                        let ok = (0..3).all(|i| {
                            let lo = o[i].min(r[i]).saturating_sub(1);
                            let hi = o[i].max(r[i]).saturating_add(1);
                            f[i] >= lo && f[i] <= hi
                        });
                        if !ok {
                            partial_blend_violation += 1;
                        }
                    }
                }
            }
        }

        eprintln!("[poly_verify] ── 验证结果 ──");
        eprintln!(
            "[poly_verify] 保护区像素: {} | final vs original 最大差值: {} (必须为 0)",
            protected_count, max_diff_protected
        );
        eprintln!(
            "[poly_verify] 原始 API 结果在保护区改动的像素: {} ({:.1}%) ← 合成兜底修掉的部分",
            raw_changed_protected,
            raw_changed_protected as f64 / protected_count as f64 * 100.0
        );
        eprintln!(
            "[poly_verify] 多边形选区内实际变化像素: {} / {} ({:.1}%)",
            edited_changed,
            edited_count,
            edited_changed as f64 / edited_count.max(1) as f64 * 100.0
        );
        eprintln!(
            "[poly_verify] 边缘半透明像素: {} | 混合越界像素: {} (必须为 0)",
            partial_count, partial_blend_violation
        );

        assert_eq!(
            max_diff_protected, 0,
            "保护区(mask alpha=255)像素必须与原图完全一致"
        );
        assert!(edited_changed > 0, "多边形选区内应有实际编辑发生");
        assert_eq!(
            partial_blend_violation, 0,
            "边缘半透明像素必须是原图与结果的线性混合"
        );
        eprintln!("[poly_verify] ✅ PASS — 多边形选区外零改动，选区内编辑生效，边缘平滑混合");
    });
}

/// Dense-polygon variant: a 25-vertex CONCAVE polygon tracing the LEFT
/// figure's silhouette (notches between the legs, between sleeve and skirt,
/// between the kimono tail and the leg — the shape a real user would click
/// out with the polygon tool). Exercises many-vertex rasterization,
/// concavity handling in the fill rule, and boundary blending along
/// irregular slanted edges.
#[test]
#[ignore]
fn polygon_dense_mask_end_to_end() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        GeminiClients::init().expect("init clients");
        let clients = gemini_client::get_clients().expect("get clients");
        eprintln!(
            "[poly25] image provider = {}, model = {}",
            clients.image_provider, clients.image_model
        );

        let src_path = out_dir().join("source.jpeg");
        let original = image_utils::load_image_from_path(src_path.to_str().unwrap())
            .expect("load source image");
        let (ow, oh) = (original.width(), original.height());

        let api_img = image_utils::resize_max_dimension(&original, 2048);
        let img_b64 = image_utils::bytes_to_base64(
            &image_utils::image_to_png_bytes(&api_img).expect("encode api image"),
        );

        // --- 25-vertex concave silhouette of the left figure (clockwise) ---
        let (mw, mh) = working_size(ow, oh);
        eprintln!("[poly25] mask working size {}x{}", mw, mh);
        let verts: Vec<(f32, f32)> = [
            (0.262, 0.118), // 发饰蝴蝶结顶
            (0.310, 0.160), // 蝴蝶结右
            (0.330, 0.255), // 右马尾上段
            (0.352, 0.395), // 右马尾/袖外缘
            (0.342, 0.520), // 右袖下缘
            (0.322, 0.555), // 凹：袖与裙摆之间
            (0.352, 0.635), // 裙摆右缘
            (0.338, 0.760), // 右侧下摆
            (0.300, 0.790), // 凹：下摆与右腿之间
            (0.296, 0.870), // 右小腿外缘
            (0.290, 0.965), // 右脚外
            (0.258, 0.968), // 右脚内
            (0.252, 0.800), // 凹：双腿之间上探
            (0.238, 0.955), // 左脚内
            (0.203, 0.948), // 左脚外
            (0.215, 0.830), // 左小腿
            (0.222, 0.770), // 凹：左腿与黑色垂尾之间
            (0.163, 0.790), // 黑色垂尾尖
            (0.150, 0.690), // 垂尾左缘
            (0.183, 0.600), // 凹：垂尾与左袖之间
            (0.150, 0.500), // 左袖外缘
            (0.153, 0.395), // 左袖上缘
            (0.165, 0.290), // 左马尾外缘
            (0.172, 0.200), // 左马尾上段
            (0.212, 0.148), // 蝴蝶结左
        ]
        .iter()
        .map(|(fx, fy)| (fx * mw as f32, fy * mh as f32))
        .collect();
        eprintln!("[poly25] 多边形顶点数: {}", verts.len());

        let mut mask = RgbaImage::from_pixel(mw, mh, Rgba([255, 255, 255, 255]));
        punch_polygon_aa(&mut mask, &verts);

        let (mut n_edit, mut n_partial) = (0u64, 0u64);
        for p in mask.pixels() {
            match p[3] {
                0 => n_edit += 1,
                255 => {}
                _ => n_partial += 1,
            }
        }
        eprintln!(
            "[poly25] 选区: {} px ({:.1}%), 抗锯齿边缘半透明像素: {}",
            n_edit,
            n_edit as f64 / (mw * mh) as f64 * 100.0,
            n_partial
        );
        assert!(n_partial > 0, "斜边应产生抗锯齿半透明像素");

        let mask_img = DynamicImage::ImageRgba8(mask);
        let mask_png = image_utils::image_to_png_bytes(&mask_img).expect("encode mask");
        std::fs::write(out_dir().join("poly25_mask.png"), &mask_png).expect("save mask");
        let mask_b64 = image_utils::bytes_to_base64(&mask_png);

        let prompt = "将选区内（画面左侧的人物）的红色服装改为宝蓝色，\
                      保持服装款式、花纹结构、褶皱与光照不变。\
                      人物的皮肤和头发颜色保持原样。\
                      画面右侧的人物和背景不得有任何改动。";
        eprintln!("[poly25] calling image API with 25-vertex polygon mask...");
        let t0 = std::time::Instant::now();
        let result_bytes = gemini_client::call_image_generation(
            &img_b64,
            prompt,
            &[],
            0.3,
            Some((ow, oh)),
            Some(&mask_b64),
        )
        .await
        .expect("image generation with dense polygon mask");
        eprintln!(
            "[poly25] API returned {} bytes in {:.1}s",
            result_bytes.len(),
            t0.elapsed().as_secs_f32()
        );
        std::fs::write(out_dir().join("poly25_raw.png"), &result_bytes)
            .expect("save raw result");

        let api_result =
            image_utils::load_image_from_bytes(&result_bytes).expect("decode api result");
        let result_resized = image_utils::resize_to_original(&api_result, (ow, oh));
        let mask_resized = image_utils::resize_to_original(&mask_img, (ow, oh));
        let final_img =
            image_utils::composite_with_mask(&original, &result_resized, &mask_resized);
        image_utils::save_png(&final_img, &out_dir().join("poly25_final.png"))
            .expect("save final");

        let orig_rgba = original.to_rgba8();
        let raw_rgba = result_resized.to_rgba8();
        let final_rgba = final_img.to_rgba8();
        let mask_rgba = mask_resized.to_rgba8();

        let mut max_diff_protected = 0u8;
        let mut raw_changed_protected = 0u64;
        let mut protected_count = 0u64;
        let mut edited_changed = 0u64;
        let mut edited_count = 0u64;
        let mut partial_count = 0u64;
        let mut partial_blend_violation = 0u64;

        for y in 0..oh {
            for x in 0..ow {
                let a = mask_rgba.get_pixel(x, y)[3];
                let o = orig_rgba.get_pixel(x, y);
                let f = final_rgba.get_pixel(x, y);
                let r = raw_rgba.get_pixel(x, y);
                let df = (0..3).map(|i| o[i].abs_diff(f[i])).max().unwrap();
                let dr = (0..3).map(|i| o[i].abs_diff(r[i])).max().unwrap();
                match a {
                    255 => {
                        protected_count += 1;
                        max_diff_protected = max_diff_protected.max(df);
                        if dr > 12 {
                            raw_changed_protected += 1;
                        }
                    }
                    0 => {
                        edited_count += 1;
                        if df > 12 {
                            edited_changed += 1;
                        }
                    }
                    _ => {
                        partial_count += 1;
                        let ok = (0..3).all(|i| {
                            let lo = o[i].min(r[i]).saturating_sub(1);
                            let hi = o[i].max(r[i]).saturating_add(1);
                            f[i] >= lo && f[i] <= hi
                        });
                        if !ok {
                            partial_blend_violation += 1;
                        }
                    }
                }
            }
        }

        eprintln!("[poly25] ── 验证结果 ──");
        eprintln!(
            "[poly25] 保护区像素: {} | final vs original 最大差值: {} (必须为 0)",
            protected_count, max_diff_protected
        );
        eprintln!(
            "[poly25] 原始 API 结果在保护区改动的像素: {} ({:.1}%) ← 合成兜底修掉的部分",
            raw_changed_protected,
            raw_changed_protected as f64 / protected_count as f64 * 100.0
        );
        eprintln!(
            "[poly25] 选区内实际变化像素: {} / {} ({:.1}%)",
            edited_changed,
            edited_count,
            edited_changed as f64 / edited_count.max(1) as f64 * 100.0
        );
        eprintln!(
            "[poly25] 边缘半透明像素: {} | 混合越界像素: {} (必须为 0)",
            partial_count, partial_blend_violation
        );

        assert_eq!(
            max_diff_protected, 0,
            "保护区(mask alpha=255)像素必须与原图完全一致"
        );
        assert!(edited_changed > 0, "选区内应有实际编辑发生");
        assert_eq!(
            partial_blend_violation, 0,
            "边缘半透明像素必须是原图与结果的线性混合"
        );
        eprintln!("[poly25] ✅ PASS — 25 顶点凹多边形：选区外零改动，选区内编辑生效，边缘平滑混合");
    });
}
