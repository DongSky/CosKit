//! End-to-end layer verification through the real edit pipeline (requires
//! configured API credentials, same as mask_verify).
//!
//! Flow: create_session → submit_edit with a rect mask (legacy single-call
//! pipeline) → wait for completion → assert the node got a proper layer
//! stack, the stack flattens to the stored node image, and hide/show of the
//! edit layer recomposites correctly. The test session is deleted afterwards.
//!
//! Run manually:
//!   cargo test --test layers_e2e -- --ignored --nocapture

use coskit::engine::{self, AppState};
use coskit::gemini_client::GeminiClients;
use coskit::image_utils;
use coskit::models::PipelineModules;
use image::{DynamicImage, Rgba, RgbaImage};
use std::path::Path;

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
#[ignore]
fn layers_end_to_end() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        GeminiClients::init().expect("init clients");

        // --- Session from the standard test image ---
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_output/source.jpeg");
        let bytes = std::fs::read(&src).expect("read source.jpeg");
        let session = engine::create_session(&bytes, "layers_e2e.jpeg").expect("create session");
        let session_id = session.id.clone();
        let root_id = session.root_id.clone();
        eprintln!("[layers_e2e] session {session_id} (will be deleted at the end)");

        // Root must carry a base layer from creation
        let root = session.nodes.get(&root_id).unwrap();
        assert_eq!(root.layers.len(), 1, "root should have exactly the base layer");
        assert_eq!(root.layers[0].kind, "base");

        let state = AppState::new();
        state.sessions.write().unwrap().insert(session_id.clone(), session);

        // --- Submit a masked edit through the real pipeline (legacy
        //     single-call path: retouch only, no agent/planner) ---
        let (ow, oh) = {
            let lock = state.sessions.read().unwrap();
            lock.get(&session_id).unwrap().original_size
        };
        let mut mask = RgbaImage::from_pixel(ow, oh, Rgba([255, 255, 255, 255]));
        for y in (oh / 10)..oh {
            for x in (ow * 17 / 100)..(ow * 40 / 100) {
                mask.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        let mask_png =
            image_utils::image_to_png_bytes(&DynamicImage::ImageRgba8(mask)).unwrap();
        let mask_b64 = image_utils::bytes_to_base64(&mask_png);

        let modules = PipelineModules {
            retouch: true,
            background: false,
            effects: false,
            agent_mode: false,
            save_intermediates: false,
            combined_mode: false,
            review_enabled: false,
        };
        let node = engine::submit_edit(
            &state,
            &session_id,
            &root_id,
            "将选区内（画面左侧的人物）的头发颜色改为金色，其余内容保持不变。",
            modules,
            Vec::new(),
            Some(mask_b64),
        )
        .expect("submit edit");
        let node_id = node.id.clone();

        // --- Wait for the background pipeline ---
        let mut status = String::new();
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            status = {
                let lock = state.sessions.read().unwrap();
                lock.get(&session_id)
                    .and_then(|s| s.nodes.get(&node_id))
                    .map(|n| n.status.clone())
                    .unwrap_or_default()
            };
            if status == "done" || status == "error" {
                break;
            }
        }
        eprintln!("[layers_e2e] pipeline status: {status}");

        let mut failures: Vec<String> = Vec::new();
        let mut check = |cond: bool, msg: &str| {
            if !cond {
                failures.push(msg.to_string());
                eprintln!("[layers_e2e] ✗ {msg}");
            } else {
                eprintln!("[layers_e2e] ✓ {msg}");
            }
        };

        check(status == "done", "管线执行完成");

        if status == "done" {
            let (layers, image_path) = {
                let lock = state.sessions.read().unwrap();
                let n = lock.get(&session_id).unwrap().nodes.get(&node_id).unwrap().clone();
                (n.layers, n.image_path)
            };
            check(layers.len() == 2, "编辑节点图层栈应为 [基础, 编辑] 两层");
            if layers.len() == 2 {
                check(layers[0].kind == "base", "第 0 层为 base");
                check(layers[1].kind == "edit", "第 1 层为 edit");
                check(!layers[1].mask_path.is_empty(), "编辑图层记录了 mask 来源");
                check(
                    layers[1].image_path != image_path,
                    "图层栅格文件独立于节点合成图（不可变约束）"
                );

                // Flatten must reproduce the stored node image
                let stored = image_utils::load_image_from_path(&image_path).unwrap();
                let imgs: Vec<DynamicImage> = layers
                    .iter()
                    .map(|l| image_utils::load_image_from_path(&l.image_path).unwrap())
                    .collect();
                let inputs: Vec<image_utils::LayerInput> = layers
                    .iter()
                    .zip(imgs.iter())
                    .map(|(l, img)| image_utils::LayerInput {
                        image: img,
                        opacity: l.opacity,
                        blend_mode: &l.blend_mode,
                        visible: l.visible,
                    })
                    .collect();
                let flat = image_utils::composite_layers(&inputs).unwrap();
                let d = max_channel_diff(&stored, &flat);
                check(d <= 1, &format!("图层栈 flatten 与节点存储图一致 (max diff {d})"));

                // Hide the edit layer → node image must revert to the base
                let edit_id = layers[1].id.clone();
                let r = engine::modify_layers(&state.sessions, &session_id, &node_id, |ls| {
                    ls.iter_mut().find(|l| l.id == edit_id).unwrap().visible = false;
                    Ok(())
                });
                check(r.is_ok(), "隐藏编辑图层 + 重合成成功");

                let (new_path, base_path) = {
                    let lock = state.sessions.read().unwrap();
                    let n = lock.get(&session_id).unwrap().nodes.get(&node_id).unwrap();
                    (n.image_path.clone(), n.layers[0].image_path.clone())
                };
                check(new_path != image_path, "重合成后节点指向独立 flatten 文件");
                let base_img = image_utils::load_image_from_path(&base_path).unwrap();
                let now_img = image_utils::load_image_from_path(&new_path).unwrap();
                let d = max_channel_diff(&base_img, &now_img);
                check(d == 0, &format!("隐藏编辑图层后与基础图层逐位一致 (max diff {d})"));

                // Show it again → back to the original flatten
                let r = engine::modify_layers(&state.sessions, &session_id, &node_id, |ls| {
                    ls.iter_mut().find(|l| l.id == edit_id).unwrap().visible = true;
                    Ok(())
                });
                check(r.is_ok(), "恢复编辑图层可见性成功");
                let restored_path = {
                    let lock = state.sessions.read().unwrap();
                    lock.get(&session_id).unwrap().nodes.get(&node_id).unwrap().image_path.clone()
                };
                let restored = image_utils::load_image_from_path(&restored_path).unwrap();
                let d = max_channel_diff(&stored, &restored);
                check(d <= 1, &format!("恢复后与原合成结果一致 (max diff {d})"));
            }
        }

        // --- Cleanup: remove the test session from disk ---
        let deleted = engine::delete_session_from_disk(&session_id);
        eprintln!("[layers_e2e] cleanup: session deleted = {deleted}");

        assert!(
            failures.is_empty(),
            "layers_e2e failures:\n{}",
            failures.join("\n")
        );
        eprintln!("[layers_e2e] ✅ PASS");
    });
}
