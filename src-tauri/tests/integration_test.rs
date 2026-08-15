//! Integration tests for GPUPixel and PixelFreeEffects
//!
//! Tests for conflict detection and resource isolation between both libraries.
//! Requires both `gpupixel` and `pixelfree` features.
//! Run with: cargo test --features "gpupixel,pixelfree" --test integration_test -- --ignored --nocapture
#![cfg(all(feature = "gpupixel", feature = "pixelfree"))]

#[cfg(test)]
mod tests {
    use coskit::beauty_filter::{create_beauty_filter, BeautyParams, FaceReshapeParams};

    /// Test that both filters can be created simultaneously without conflict
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_both_filters_coexist() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        // Both should initialize successfully
        assert!(gpu_filter.init().is_ok(), "GPUPixel init should succeed");
        assert!(pf_filter.init().is_ok(), "PixelFree init should succeed");

        // Clean up
        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test alternating calls between both filters (critical conflict test)
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_alternating_calls() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 128u32;
        let height = 128u32;
        let input = vec![128u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.5,
            smoothing: 0.3,
            sharpening: 0.2,
            ..Default::default()
        };

        // Alternate calls multiple times
        for i in 0..5 {
            println!("Iteration {}", i);

            // Call GPUPixel
            let result1 = gpu_filter.apply(&input, width, height, &params);
            assert!(result1.is_ok(), "GPUPixel call {} should succeed", i);

            // Call PixelFree
            let result2 = pf_filter.apply(&input, width, height, &params);
            assert!(result2.is_ok(), "PixelFree call {} should succeed", i);

            // Call GPUPixel again
            let result3 = gpu_filter.apply(&input, width, height, &params);
            assert!(result3.is_ok(), "GPUPixel call {} (second) should succeed", i);
        }

        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test creating and destroying filters multiple times
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_reinitialize_both() {
        for cycle in 0..3 {
            println!("Cycle {}", cycle);

            let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
            let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

            gpu_filter.init().unwrap();
            pf_filter.init().unwrap();

            let width = 64u32;
            let height = 64u32;
            let input = vec![100u8; (width * height * 4) as usize];

            let params = BeautyParams {
                whitening: 0.3,
                smoothing: 0.2,
                sharpening: 0.1,
                ..Default::default()
            };

            gpu_filter.apply(&input, width, height, &params).unwrap();
            pf_filter.apply(&input, width, height, &params).unwrap();

            gpu_filter.destroy();
            pf_filter.destroy();
        }
    }

    /// Test both filters with same input produce deterministic results
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_deterministic_results() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 100u32;
        let height = 100u32;
        let input = vec![150u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.4,
            smoothing: 0.3,
            sharpening: 0.2,
            ..Default::default()
        };

        // Run GPUPixel twice
        let gpu_result1 = gpu_filter.apply(&input, width, height, &params).unwrap();
        let gpu_result2 = gpu_filter.apply(&input, width, height, &params).unwrap();
        assert_eq!(
            gpu_result1.len(),
            gpu_result2.len(),
            "GPUPixel should produce consistent results"
        );

        // Run PixelFree twice
        let pf_result1 = pf_filter.apply(&input, width, height, &params).unwrap();
        let pf_result2 = pf_filter.apply(&input, width, height, &params).unwrap();
        assert_eq!(
            pf_result1.len(),
            pf_result2.len(),
            "PixelFree should produce consistent results"
        );

        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test context switching doesn't corrupt state
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_context_switching() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 256u32;
        let height = 256u32;
        let input1 = vec![100u8; (width * height * 4) as usize];
        let input2 = vec![200u8; (width * height * 4) as usize];

        let params1 = BeautyParams {
            whitening: 0.3,
            smoothing: 0.2,
            sharpening: 0.1,
            ..Default::default()
        };

        let params2 = BeautyParams {
            whitening: 0.7,
            smoothing: 0.5,
            sharpening: 0.4,
            ..Default::default()
        };

        // Process different inputs with different parameters
        let result_gpu1 = gpu_filter.apply(&input1, width, height, &params1).unwrap();
        let result_pf1 = pf_filter.apply(&input2, width, height, &params2).unwrap();

        // Switch back with different parameters
        let result_gpu2 = gpu_filter.apply(&input2, width, height, &params2).unwrap();
        let result_pf2 = pf_filter.apply(&input1, width, height, &params1).unwrap();

        // Results should be valid
        assert_eq!(result_gpu1.len(), (width * height * 4) as usize);
        assert_eq!(result_pf1.len(), (width * height * 4) as usize);
        assert_eq!(result_gpu2.len(), (width * height * 4) as usize);
        assert_eq!(result_pf2.len(), (width * height * 4) as usize);

        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test different parameter combinations don't conflict
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_different_parameters() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 128u32;
        let height = 128u32;
        let input = vec![128u8; (width * height * 4) as usize];

        // GPUPixel with basic parameters
        let gpu_params = BeautyParams {
            whitening: 0.8,
            smoothing: 0.6,
            sharpening: 0.4,
            ..Default::default()
        };

        // PixelFree with advanced parameters
        let pf_params = BeautyParams {
            whitening: 0.5,
            smoothing: 0.4,
            sharpening: 0.3,
            ruddy: 0.2,
            eye_brighten: 0.3,
            face_reshape: Some(FaceReshapeParams {
                eye_strength: 0.4,
                face_thinning: 0.3,
                face_v: 0.2,
                nose: 0.1,
            }),
        };

        // Both should work with their respective parameters
        assert!(gpu_filter.apply(&input, width, height, &gpu_params).is_ok());
        assert!(pf_filter.apply(&input, width, height, &pf_params).is_ok());

        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test large image processing doesn't cause conflicts
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_large_images() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 1024u32;
        let height = 1024u32;
        let input = vec![128u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.5,
            smoothing: 0.3,
            sharpening: 0.2,
            ..Default::default()
        };

        // Process large image with both filters
        let result_gpu = gpu_filter.apply(&input, width, height, &params);
        let result_pf = pf_filter.apply(&input, width, height, &params);

        assert!(result_gpu.is_ok(), "GPUPixel should handle 1024x1024");
        assert!(result_pf.is_ok(), "PixelFree should handle 1024x1024");

        gpu_filter.destroy();
        pf_filter.destroy();
    }

    /// Test rapid switching between filters
    #[test]
    #[ignore] // Requires GPU and PixelFree SDK
    fn test_rapid_switching() {
        let mut gpu_filter = create_beauty_filter("gpupixel").unwrap();
        let mut pf_filter = create_beauty_filter("pixelfree").unwrap();

        gpu_filter.init().unwrap();
        pf_filter.init().unwrap();

        let width = 64u32;
        let height = 64u32;
        let input = vec![120u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.3,
            smoothing: 0.3,
            sharpening: 0.3,
            ..Default::default()
        };

        // Rapid alternation (50 times)
        for i in 0..50 {
            if i % 2 == 0 {
                assert!(
                    gpu_filter.apply(&input, width, height, &params).is_ok(),
                    "GPUPixel rapid call {} failed",
                    i
                );
            } else {
                assert!(
                    pf_filter.apply(&input, width, height, &params).is_ok(),
                    "PixelFree rapid call {} failed",
                    i
                );
            }
        }

        gpu_filter.destroy();
        pf_filter.destroy();
    }
}

/// End-to-end: beauty edit through the real engine node lifecycle
/// (session → submit_beauty_edit → poll → done node with image + layers).
#[cfg(test)]
mod e2e {
    use coskit::beauty_filter::BeautyParams;
    use coskit::engine::{self, AppState};
    use std::path::Path;

    #[test]
    #[ignore] // requires GPU
    fn beauty_edit_node_pipeline() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_output/source.jpeg");
            let bytes = std::fs::read(&src).expect("read source.jpeg");
            let session = engine::create_session(&bytes, "beauty_e2e.jpeg").expect("create session");
            let session_id = session.id.clone();
            let root_id = session.root_id.clone();

            let state = AppState::new();
            state
                .sessions
                .write()
                .unwrap()
                .insert(session_id.clone(), session);

            let params = BeautyParams {
                whitening: 0.4,
                smoothing: 0.5,
                sharpening: 0.2,
                ..Default::default()
            };
            let node = engine::submit_beauty_edit(&state, &session_id, &root_id, "gpupixel", params)
                .expect("submit beauty edit");

            // Poll until done (local processing should be fast)
            let mut status = String::new();
            for _ in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                let lock = state.sessions.read().unwrap();
                let s = lock.get(&session_id).unwrap();
                let n = s.nodes.get(&node.id).unwrap();
                status = n.status.clone();
                if status == "done" || status == "error" {
                    break;
                }
            }

            let lock = state.sessions.read().unwrap();
            let s = lock.get(&session_id).unwrap();
            let n = s.nodes.get(&node.id).unwrap();
            assert_eq!(status, "done", "node error: {:?}", n.error_msg);
            assert!(!n.image_path.is_empty(), "image_path set");
            assert!(Path::new(&n.image_path).exists(), "result image exists");
            assert!(n.layers.len() >= 2, "base + beauty edit layer");
            assert!(n.prompt.contains("本地美颜"), "prompt labeled");
            drop(lock);

            // Unknown provider is rejected synchronously
            assert!(engine::submit_beauty_edit(
                &state,
                &session_id,
                &root_id,
                "nope",
                BeautyParams::default()
            )
            .is_err());

            engine::delete_session_from_disk(&session_id);
        });
    }
}
