//! Unit tests for PixelFree beauty filter
//!
//! Note: These tests require the `pixelfree` feature and SDK authorization files.
//! Run with: cargo test --features pixelfree --test pixelfree_test -- --ignored --nocapture
#![cfg(feature = "pixelfree")]

#[cfg(test)]
mod tests {
    use coskit::beauty_filter::{BeautyFilter, BeautyParams, FaceReshapeParams};
    use coskit::beauty_pixelfree::PixelFreeFilter;

    #[test]
    #[ignore] // Requires PixelFree SDK and authorization
    fn test_pixelfree_init() {
        let mut filter = PixelFreeFilter::new();
        let result = filter.init();
        assert!(result.is_ok(), "Filter initialization should succeed");
        filter.destroy();
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_basic_beauty() {
        let mut filter = PixelFreeFilter::new();
        filter.init().unwrap();

        let width = 100u32;
        let height = 100u32;
        let input = vec![128u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.5,
            smoothing: 0.3,
            sharpening: 0.2,
            ruddy: 0.1,
            eye_brighten: 0.0,
            face_reshape: None,
        };

        let result = filter.apply(&input, width, height, &params);
        assert!(result.is_ok(), "Apply should succeed");

        let output = result.unwrap();
        assert_eq!(output.len(), input.len(), "Output size should match input");

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_face_reshape() {
        let mut filter = PixelFreeFilter::new();
        filter.init().unwrap();

        let width = 200u32;
        let height = 200u32;
        let input = vec![150u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.3,
            smoothing: 0.3,
            sharpening: 0.1,
            ruddy: 0.2,
            eye_brighten: 0.4,
            face_reshape: Some(FaceReshapeParams {
                eye_strength: 0.3,
                face_thinning: 0.2,
                face_v: 0.1,
                nose: 0.15,
            }),
        };

        let result = filter.apply(&input, width, height, &params);
        assert!(result.is_ok(), "Apply with face reshape should succeed");

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_parameter_range() {
        let mut filter = PixelFreeFilter::new();
        filter.init().unwrap();

        let width = 64u32;
        let height = 64u32;
        let input = vec![100u8; (width * height * 4) as usize];

        // Test min values
        let params_min = BeautyParams {
            whitening: 0.0,
            smoothing: 0.0,
            sharpening: 0.0,
            ruddy: 0.0,
            eye_brighten: 0.0,
            face_reshape: None,
        };
        assert!(filter.apply(&input, width, height, &params_min).is_ok());

        // Test max values
        let params_max = BeautyParams {
            whitening: 1.0,
            smoothing: 1.0,
            sharpening: 1.0,
            ruddy: 1.0,
            eye_brighten: 1.0,
            face_reshape: Some(FaceReshapeParams {
                eye_strength: 1.0,
                face_thinning: 1.0,
                face_v: 1.0,
                nose: 1.0,
            }),
        };
        assert!(filter.apply(&input, width, height, &params_max).is_ok());

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_multiple_apply() {
        let mut filter = PixelFreeFilter::new();
        filter.init().unwrap();

        let width = 128u32;
        let height = 128u32;
        let input = vec![120u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.4,
            smoothing: 0.3,
            sharpening: 0.2,
            ruddy: 0.1,
            eye_brighten: 0.2,
            face_reshape: None,
        };

        // Apply multiple times
        let result1 = filter.apply(&input, width, height, &params).unwrap();
        let result2 = filter.apply(&input, width, height, &params).unwrap();

        assert_eq!(result1.len(), result2.len());

        filter.destroy();
    }

    #[test]
    fn test_pixelfree_init_without_sdk() {
        // This test should fail gracefully without SDK
        let mut filter = PixelFreeFilter::new();
        let result = filter.init();

        if result.is_err() {
            println!("PixelFree SDK not available: {}", result.unwrap_err());
        }
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_reinitialize() {
        let mut filter = PixelFreeFilter::new();

        filter.init().unwrap();
        filter.destroy();

        let result = filter.init();
        assert!(result.is_ok(), "Reinitialization should succeed");

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires PixelFree SDK
    fn test_pixelfree_different_image_sizes() {
        let mut filter = PixelFreeFilter::new();
        filter.init().unwrap();

        let params = BeautyParams {
            whitening: 0.3,
            smoothing: 0.3,
            sharpening: 0.2,
            ruddy: 0.1,
            eye_brighten: 0.2,
            face_reshape: None,
        };

        let sizes = vec![(64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)];

        for (width, height) in sizes {
            let input = vec![128u8; (width * height * 4) as usize];
            let result = filter.apply(&input, width, height, &params);
            assert!(result.is_ok(), "Should handle {}x{} image", width, height);
        }

        filter.destroy();
    }
}
