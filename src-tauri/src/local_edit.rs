//! Local, reversible raster commands. Pixel work runs outside the session lock.
use crate::{
    engine, image_utils,
    models::{EditNode, Layer, Session},
    settings,
};
use image::{DynamicImage, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const MAX_PIXELS: u64 = 40_000_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditRequest {
    pub operation: String,
    #[serde(default)]
    pub layer_id: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    #[serde(default)]
    pub mask: Option<String>,
}

fn number(p: &Value, key: &str, default: f32, min: f32, max: f32) -> Result<f32, String> {
    let n = match p.get(key) {
        None => default,
        Some(v) => v.as_f64().ok_or_else(|| format!("无效参数: {key}"))? as f32,
    };
    if !n.is_finite() || n < min || n > max {
        return Err(format!("{key} 必须在 {min}–{max} 之间"));
    }
    Ok(n)
}

fn check_size(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err("图像尺寸必须为正且不超过 4000 万像素".into());
    }
    Ok(())
}

fn coverage(req: &EditRequest, w: u32, h: u32) -> Result<Vec<f32>, String> {
    let mut weights = vec![1.; (w as usize) * (h as usize)];
    if let Some(mask) = &req.mask {
        if mask.len() > 80_000_000 {
            return Err("选区文件过大".into());
        }
        let bytes =
            image_utils::base64_to_bytes(mask.split_once(',').map_or(mask.as_str(), |(_, b)| b))?;
        let m = image::load_from_memory(&bytes)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        if m.dimensions() != (w, h) {
            return Err("选区尺寸与当前画布不一致，请重新选择".into());
        }
        for (v, px) in weights.iter_mut().zip(m.pixels()) {
            *v = 1. - px[3] as f32 / 255.;
        }
    }
    Ok(weights)
}

/// Apply a validated pixel command, retaining exact RGBA outside its coverage.
pub fn raster_edit(input: &RgbaImage, req: &EditRequest) -> Result<RgbaImage, String> {
    let (w, h) = input.dimensions();
    check_size(w, h)?;
    if req.points.len() > 20_000
        || req
            .points
            .iter()
            .flatten()
            .any(|p| !p.is_finite() || p.abs() > 100_000.)
    {
        return Err("笔画坐标无效或过长".into());
    }
    let p = &req.params;
    let mut out = input.clone();
    let mut weights = coverage(req, w, h)?;
    let brush = matches!(
        req.operation.as_str(),
        "brush" | "erase" | "clone" | "heal" | "spot" | "dodge" | "burn" | "sponge"
    );
    if brush {
        if req.points.is_empty() {
            return Err("请在画布上绘制笔画".into());
        }
        let radius = number(p, "size", 30., 1., 1000.)? / 2.;
        let strength = number(p, "strength", 1., 0.01, 1.)?;
        let hardness = number(p, "hardness", 0.6, 0., 1.)?;
        let mut stroke = vec![0f32; weights.len()];
        let mut dab_count = 0usize;
        let mut work = 0u64;
        let mut dab = |cx: f32, cy: f32| -> Result<(), String> {
            dab_count += 1;
            if dab_count > 100_000 {
                return Err("笔画过长，请分段绘制".into());
            }
            let x0 = (cx - radius).floor().max(0.) as u32;
            let y0 = (cy - radius).floor().max(0.) as u32;
            let x1 = ((cx + radius).ceil().max(0.) as u32).min(w);
            let y1 = ((cy + radius).ceil().max(0.) as u32).min(h);
            work = work.saturating_add(
                u64::from(x1.saturating_sub(x0)) * u64::from(y1.saturating_sub(y0)),
            );
            if work > 100_000_000 {
                return Err("单次笔画计算量过大，请缩短笔画或减小画笔".into());
            }
            for y in y0..y1 {
                for x in x0..x1 {
                    let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt()
                        / radius;
                    let a = if d <= hardness {
                        1.
                    } else {
                        ((1. - d) / (1. - hardness).max(0.001)).clamp(0., 1.)
                    } * strength;
                    let i = (y * w + x) as usize;
                    stroke[i] = stroke[i].max(a);
                }
            }
            Ok(())
        };
        dab(req.points[0][0], req.points[0][1])?;
        for pair in req.points.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let n = (((b[0] - a[0]).hypot(b[1] - a[1]) / (radius * 0.3).max(1.)).ceil() as usize)
                .max(1);
            if n > 100_000 {
                return Err("笔画过长".into());
            }
            for j in 1..=n {
                let t = j as f32 / n as f32;
                dab(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)?;
            }
        }
        for (v, s) in weights.iter_mut().zip(stroke) {
            *v *= s;
        }
    }
    match req.operation.as_str() {
        "brush" => {
            let color = p
                .get("color")
                .and_then(Value::as_str)
                .unwrap_or("#ffffff")
                .trim_start_matches('#');
            if color.len() != 6 || !color.is_ascii() {
                return Err("颜色必须为 #RRGGBB".into());
            }
            let value = u32::from_str_radix(color, 16).map_err(|_| "颜色格式无效")?;
            for px in out.pixels_mut() {
                *px = Rgba([(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]);
            }
        }
        "erase" => {
            for px in out.pixels_mut() {
                px[3] = 0;
            }
        }
        "clone" | "heal" => {
            let sx = number(p, "source_x", 0., 0., w.saturating_sub(1) as f32)?;
            let sy = number(p, "source_y", 0., 0., h.saturating_sub(1) as f32)?;
            let dx = sx - req.points[0][0];
            let dy = sy - req.points[0][1];
            for y in 0..h {
                for x in 0..w {
                    let xx = (x as f32 + dx).round() as i64;
                    let yy = (y as f32 + dy).round() as i64;
                    if xx >= 0 && yy >= 0 && xx < w as i64 && yy < h as i64 {
                        out.put_pixel(x, y, *input.get_pixel(xx as u32, yy as u32));
                    } else {
                        weights[(y * w + x) as usize] = 0.;
                    }
                }
            }
            if req.operation == "heal" {
                let mut x0 = w;
                let mut y0 = h;
                let mut x1 = 0;
                let mut y1 = 0;
                for y in 0..h {
                    for x in 0..w {
                        if weights[(y * w + x) as usize] > 0. {
                            x0 = x0.min(x);
                            y0 = y0.min(y);
                            x1 = x1.max(x + 1);
                            y1 = y1.max(y + 1);
                        }
                    }
                }
                if x1 == 0 {
                    return Ok(input.clone());
                }
                x0 = x0.saturating_sub(2);
                y0 = y0.saturating_sub(2);
                x1 = (x1 + 2).min(w);
                y1 = (y1 + 2).min(h);
                let (rw, rh) = (x1 - x0, y1 - y0);
                if u64::from(rw) * u64::from(rh) > 4_000_000 {
                    return Err("修复笔画区域过大，请分段处理".into());
                }
                let mut src = Vec::new();
                let mut dst = Vec::new();
                let mut mask = Vec::new();
                for y in y0..y1 {
                    for x in x0..x1 {
                        src.extend(out.get_pixel(x, y).0[..3].iter().map(|v| *v as f32 / 255.));
                        dst.extend(
                            input.get_pixel(x, y).0[..3]
                                .iter()
                                .map(|v| *v as f32 / 255.),
                        );
                        mask.push(weights[(y * w + x) as usize] > 0.);
                    }
                }
                let result = photocraft_retouch::poisson::seamless_clone(
                    rw as usize,
                    rh as usize,
                    3,
                    &src,
                    &dst,
                    &mask,
                );
                for y in y0..y1 {
                    for x in x0..x1 {
                        for c in 0..3 {
                            out.get_pixel_mut(x, y)[c] =
                                (result[(((y - y0) * rw + x - x0) * 3) as usize + c].clamp(0., 1.)
                                    * 255.)
                                    .round() as u8;
                        }
                        out.get_pixel_mut(x, y)[3] = input.get_pixel(x, y)[3];
                    }
                }
            }
        }
        "spot" | "inpaint" => {
            if req.operation == "inpaint" && req.mask.is_none() {
                return Err("内容感知填充需要选区".into());
            }
            let hole: Vec<bool> = weights.iter().map(|v| *v > 0.).collect();
            let count = hole.iter().filter(|v| **v).count();
            if count == 0 || count == hole.len() {
                return Err("请选择局部区域，并保留周围可供采样的像素".into());
            }
            // Bound the expensive solver to a context region around the actual repair.
            let mut x0 = w;
            let mut y0 = h;
            let mut x1 = 0;
            let mut y1 = 0;
            for y in 0..h {
                for x in 0..w {
                    if hole[(y * w + x) as usize] {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x + 1);
                        y1 = y1.max(y + 1);
                    }
                }
            }
            x0 = x0.saturating_sub(64);
            y0 = y0.saturating_sub(64);
            x1 = (x1 + 64).min(w);
            y1 = (y1 + 64).min(h);
            let (rw, rh) = (x1 - x0, y1 - y0);
            if u64::from(rw) * u64::from(rh) > 4_000_000 {
                return Err("单次修复区域过大，请分区域处理（最多 400 万像素）".into());
            }
            let mut floats = Vec::with_capacity((rw * rh * 3) as usize);
            let mut local_mask = Vec::with_capacity((rw * rh) as usize);
            for y in y0..y1 {
                for x in x0..x1 {
                    let px = input.get_pixel(x, y);
                    floats.extend(px.0[..3].iter().map(|v| *v as f32 / 255.));
                    local_mask.push(hole[(y * w + x) as usize]);
                }
            }
            let result = photocraft_retouch::inpaint::complete(
                rw as usize,
                rh as usize,
                3,
                &floats,
                &local_mask,
                &Default::default(),
            )
            .ok_or("无法找到足够的修复样本，请缩小选区")?;
            for y in y0..y1 {
                for x in x0..x1 {
                    for c in 0..3 {
                        out.get_pixel_mut(x, y)[c] =
                            (result[(((y - y0) * rw + x - x0) * 3) as usize + c].clamp(0., 1.)
                                * 255.)
                                .round() as u8;
                    }
                }
            }
        }
        "dodge" | "burn" | "sponge" => {
            for px in out.pixels_mut() {
                let rgb = [
                    px[0] as f32 / 255.,
                    px[1] as f32 / 255.,
                    px[2] as f32 / 255.,
                ];
                let result = if req.operation == "sponge" {
                    photocraft_retouch::retouch::sponge(rgb, 0.6, false, true)
                } else {
                    photocraft_retouch::retouch::dodge_burn(
                        rgb,
                        0.6,
                        Default::default(),
                        req.operation == "burn",
                        true,
                    )
                };
                for c in 0..3 {
                    px[c] = (result[c].clamp(0., 1.) * 255.).round() as u8;
                }
            }
        }
        "adjust" => {
            let brightness = number(p, "brightness", 0., -100., 100.)? / 100.;
            let contrast = number(p, "contrast", 0., -100., 100.)? / 100. + 1.;
            let saturation = number(p, "saturation", 0., -100., 100.)? / 100. + 1.;
            let exposure = 2f32.powf(number(p, "exposure", 0., -4., 4.)?);
            for px in out.pixels_mut() {
                let rgb = [
                    px[0] as f32 / 255.,
                    px[1] as f32 / 255.,
                    px[2] as f32 / 255.,
                ];
                let lum = photocraft_retouch::retouch::luma(rgb);
                for c in 0..3 {
                    px[c] = ((((lum + (rgb[c] - lum) * saturation) * exposure - 0.5) * contrast
                        + 0.5
                        + brightness)
                        .clamp(0., 1.)
                        * 255.)
                        .round() as u8;
                }
            }
        }
        "blur" => {
            out = image::imageops::blur(input, number(p, "radius", 2., 0.1, 30.)?);
        }
        "sharpen" => {
            out = image::imageops::unsharpen(input, number(p, "radius", 2., 0.1, 30.)?, 3);
        }
        _ => return Err("未知本地像素命令".into()),
    }
    for ((px, original), a) in out.pixels_mut().zip(input.pixels()).zip(weights) {
        if a == 0. {
            *px = *original;
            continue;
        }
        if req.operation == "brush" {
            let alpha = a + (original[3] as f32 / 255.) * (1. - a);
            for c in 0..3 {
                px[c] = ((px[c] as f32 * a
                    + original[c] as f32 * (original[3] as f32 / 255.) * (1. - a))
                    / alpha.max(0.00001))
                .round() as u8;
            }
            px[3] = (alpha * 255.).round() as u8;
        } else {
            for c in 0..4 {
                px[c] = (original[c] as f32 * (1. - a) + px[c] as f32 * a).round() as u8;
            }
        }
    }
    Ok(out)
}

fn geometry(img: &DynamicImage, req: &EditRequest) -> Result<DynamicImage, String> {
    let p = &req.params;
    Ok(match req.operation.as_str() {
        "rotate" => match number(p, "angle", 90., 90., 270.)? as u32 {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => return Err("旋转角度必须为 90/180/270".into()),
        },
        "flip_h" => img.fliph(),
        "flip_v" => img.flipv(),
        "resize" => {
            let w = number(p, "width", img.width() as f32, 1., 20000.)? as u32;
            let h = number(p, "height", img.height() as f32, 1., 20000.)? as u32;
            check_size(w, h)?;
            img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        }
        "crop" => {
            let x = number(p, "x", 0., 0., img.width().saturating_sub(1) as f32)? as u32;
            let y = number(p, "y", 0., 0., img.height().saturating_sub(1) as f32)? as u32;
            let w = number(
                p,
                "width",
                (img.width() - x) as f32,
                1.,
                (img.width() - x) as f32,
            )? as u32;
            let h = number(
                p,
                "height",
                (img.height() - y) as f32,
                1.,
                (img.height() - y) as f32,
            )? as u32;
            img.crop_imm(x, y, w, h)
        }
        _ => return Err("未知几何命令".into()),
    })
}

pub fn prepare_node(
    parent: &EditNode,
    req: &EditRequest,
    dir: &std::path::Path,
) -> Result<EditNode, String> {
    if parent.status != "done" {
        return Err("请等待当前操作完成".into());
    }
    let mut node = parent.clone();
    engine::ensure_layers(&mut node);
    if node.layers.is_empty() {
        return Err("没有可编辑图层".into());
    }
    node.id = uuid::Uuid::new_v4().to_string();
    node.parent_id = Some(parent.id.clone());
    node.children.clear();
    node.metadata.clear();
    node.mask_image_path.clear();
    node.error_msg = None;
    node.created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let label = match req.operation.as_str() {
        "brush" => "画笔",
        "erase" => "橡皮",
        "clone" => "仿制图章",
        "heal" => "修复画笔",
        "spot" => "污点修复",
        "inpaint" => "内容感知填充",
        "dodge" => "减淡",
        "burn" => "加深",
        "sponge" => "海绵",
        "adjust" => "光线与色彩",
        "blur" => "高斯模糊",
        "sharpen" => "锐化",
        "crop" => "裁剪",
        "resize" => "调整尺寸",
        "rotate" => "旋转",
        "flip_h" => "水平翻转",
        "flip_v" => "垂直翻转",
        "layer_new" => "新建图层",
        "layer_duplicate" => "复制图层",
        "layer_delete" => "删除图层",
        "layer_reorder" => "图层排序",
        "layer_props" => "图层属性",
        _ => &req.operation,
    };
    node.prompt = format!("本地 · {label}");
    node.note = "本地编辑已保存，可撤销或从此处继续 AI 编辑。".into();
    let index = if req.layer_id.is_empty() {
        node.layers.len() - 1
    } else {
        node.layers
            .iter()
            .position(|l| l.id == req.layer_id)
            .ok_or("目标图层不存在")?
    };
    let is_geometry = matches!(
        req.operation.as_str(),
        "rotate" | "flip_h" | "flip_v" | "resize" | "crop"
    );
    if node.layers[index].locked
        && !matches!(
            req.operation.as_str(),
            "layer_props" | "layer_new" | "layer_duplicate"
        )
    {
        return Err("目标图层已锁定".into());
    }
    let base = image_utils::load_image_from_path(&parent.image_path)?;
    check_size(base.width(), base.height())?;
    if u64::from(base.width()) * u64::from(base.height()) * (node.layers.len() as u64 + 1)
        > 160_000_000
    {
        return Err("当前图层总像素量过大，请缩小图像或减少图层".into());
    }
    match req.operation.as_str() {
        "layer_new" => {
            if node.layers.len() >= 64 {
                return Err("最多支持 64 个图层".into());
            }
            let path = dir.join(format!("{}_new.png", node.id));
            image_utils::save_png(&DynamicImage::new_rgba8(base.width(), base.height()), &path)?;
            node.layers.push(Layer::new(
                "paint",
                "空白图层",
                path.to_string_lossy().into(),
            ));
        }
        "layer_duplicate" => {
            if node.layers.len() >= 64 {
                return Err("最多支持 64 个图层".into());
            }
            let mut layer = node.layers[index].clone();
            layer.id = uuid::Uuid::new_v4().to_string();
            layer.name.push_str(" 副本");
            layer.locked = false;
            node.layers.insert(index + 1, layer);
        }
        "layer_delete" => {
            if node.layers.len() == 1 {
                return Err("不能删除最后一个图层".into());
            }
            node.layers.remove(index);
        }
        "layer_reorder" => {
            let target = number(
                &req.params,
                "index",
                index as f32,
                0.,
                (node.layers.len() - 1) as f32,
            )? as usize;
            let layer = node.layers.remove(index);
            node.layers.insert(target, layer);
        }
        "layer_props" => {
            let l = &mut node.layers[index];
            let p = &req.params;
            if l.locked
                && p.as_object()
                    .is_none_or(|m| m.len() != 1 || !m.contains_key("locked"))
            {
                return Err("请先解锁图层".into());
            }
            if let Some(v) = p.get("locked").and_then(Value::as_bool) {
                l.locked = v;
            }
            if let Some(v) = p.get("visible").and_then(Value::as_bool) {
                l.visible = v;
            }
            if let Some(v) = p.get("name").and_then(Value::as_str) {
                if v.trim().is_empty() || v.len() > 200 {
                    return Err("图层名称长度应为 1–200 字节".into());
                }
                l.name = v.trim().into();
            }
            l.opacity = number(p, "opacity", l.opacity, 0., 1.)?;
            if let Some(v) = p.get("blend_mode").and_then(Value::as_str) {
                if !["normal", "multiply", "screen", "overlay"].contains(&v) {
                    return Err("不支持的混合模式".into());
                }
                l.blend_mode = v.into();
            }
        }
        _ => {
            if is_geometry && node.layers.iter().any(|l| l.locked) {
                return Err("几何变换需要先解锁全部图层".into());
            }
            for (i, l) in node.layers.iter_mut().enumerate() {
                if !is_geometry && i != index {
                    continue;
                }
                if !is_geometry && !l.visible {
                    return Err("请先显示目标图层".into());
                }
                let img = image_utils::load_image_from_path(&l.image_path)?;
                let result = if is_geometry {
                    geometry(&img, req)?
                } else {
                    DynamicImage::ImageRgba8(raster_edit(&img.to_rgba8(), req)?)
                };
                let path = dir.join(format!("{}_layer_{i}.png", node.id));
                image_utils::save_png(&result, &path)?;
                l.image_path = path.to_string_lossy().into();
                l.mask_path.clear();
            }
        }
    }
    let images: Result<Vec<_>, _> = node
        .layers
        .iter()
        .map(|l| image_utils::load_image_from_path(&l.image_path))
        .collect();
    let images = images?;
    let inputs: Vec<_> = node
        .layers
        .iter()
        .zip(images.iter())
        .map(|(l, img)| image_utils::LayerInput {
            image: img,
            opacity: l.opacity,
            blend_mode: &l.blend_mode,
            visible: l.visible,
        })
        .collect();
    let flat = image_utils::composite_layers(&inputs)?;
    let path = dir.join(format!("{}_local.png", node.id));
    image_utils::save_png(&flat, &path)?;
    let thumb = dir.join(format!("{}_thumb.jpg", node.id));
    image_utils::make_thumbnail(&flat, &thumb)?;
    node.image_path = path.to_string_lossy().into();
    node.thumbnail_path = thumb.to_string_lossy().into();
    node.metadata
        .insert("canvas_size".into(), json!([flat.width(), flat.height()]));
    node.metadata
        .insert("local_operation".into(), json!(req.operation));
    Ok(node)
}

pub fn commit_node(session: &mut Session, node: EditNode) -> Result<(), String> {
    let parent = node.parent_id.as_ref().ok_or("缺少父节点")?;
    if !session.nodes.contains_key(parent) {
        return Err("父节点已被删除".into());
    }
    let mut next = session.clone();
    next.nodes
        .get_mut(parent)
        .ok_or("父节点不存在")?
        .children
        .push(node.id.clone());
    let id = node.id.clone();
    next.nodes.insert(id.clone(), node);
    next.active_path = engine::compute_active_path(&next, &id);
    let dir = settings::data_dir().join(&session.id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
    let tmp = dir.join("session.json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("保存失败: {e}"))?;
    std::fs::rename(tmp, dir.join("session.json")).map_err(|e| format!("保存失败: {e}"))?;
    *session = next;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn apply_local_edit(
    state: tauri::State<'_, engine::AppState>,
    session_id: String,
    node_id: String,
    request: EditRequest,
) -> Result<Value, String> {
    let parent = {
        let sessions = state.sessions.read().map_err(|e| e.to_string())?;
        sessions
            .get(&session_id)
            .and_then(|s| s.nodes.get(&node_id))
            .cloned()
            .ok_or("节点不存在")?
    };
    let dir = settings::data_dir().join(&session_id);
    let node = tauri::async_runtime::spawn_blocking(move || prepare_node(&parent, &request, &dir))
        .await
        .map_err(|e| format!("本地编辑失败: {e}"))??;
    let result = node.to_dict();
    let mut sessions = state.sessions.write().map_err(|e| e.to_string())?;
    let session = sessions.get_mut(&session_id).ok_or("会话已被删除")?;
    commit_node(session, node)?;
    Ok(result)
}
