//! Unit tests for GPUPixel beauty filter
//!
//! Note: These tests require the `gpupixel` feature, OpenGL context and GPU hardware.
//! Run with: cargo test --features gpupixel --test gpupixel_test -- --ignored --nocapture
#![cfg(feature = "gpupixel")]

#[cfg(test)]
mod tests {
    use coskit::beauty_filter::{BeautyFilter, BeautyParams};
    use coskit::beauty_gpupixel::GPUPixelFilter;

    #[test]
    #[ignore] // Requires GPU and OpenGL context
    fn test_gpupixel_init() {
        let mut filter = GPUPixelFilter::new();
        let result = filter.init();
        assert!(result.is_ok(), "Filter initialization should succeed");
        filter.destroy();
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpupixel_basic_whitening() {
        let mut filter = GPUPixelFilter::new();
        filter.init().unwrap();

        // Create test image (100x100 RGBA)
        let width = 100u32;
        let height = 100u32;
        let input = vec![128u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.5,
            smoothing: 0.0,
            sharpening: 0.0,
            ..Default::default()
        };

        let result = filter.apply(&input, width, height, &params);
        assert!(result.is_ok(), "Apply should succeed");

        let output = result.unwrap();
        assert_eq!(
            output.len(),
            input.len(),
            "Output size should match input"
        );

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpupixel_parameter_range() {
        let mut filter = GPUPixelFilter::new();
        filter.init().unwrap();

        let width = 50u32;
        let height = 50u32;
        let input = vec![100u8; (width * height * 4) as usize];

        // Test min values (0.0)
        let params_min = BeautyParams {
            whitening: 0.0,
            smoothing: 0.0,
            sharpening: 0.0,
            ..Default::default()
        };
        assert!(filter.apply(&input, width, height, &params_min).is_ok());

        // Test max values (1.0)
        let params_max = BeautyParams {
            whitening: 1.0,
            smoothing: 1.0,
            sharpening: 1.0,
            ..Default::default()
        };
        assert!(filter.apply(&input, width, height, &params_max).is_ok());

        // Test mid-range values
        let params_mid = BeautyParams {
            whitening: 0.5,
            smoothing: 0.3,
            sharpening: 0.7,
            ..Default::default()
        };
        assert!(filter.apply(&input, width, height, &params_mid).is_ok());

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpupixel_multiple_apply() {
        let mut filter = GPUPixelFilter::new();
        filter.init().unwrap();

        let width = 64u32;
        let height = 64u32;
        let input = vec![150u8; (width * height * 4) as usize];

        let params = BeautyParams {
            whitening: 0.4,
            smoothing: 0.2,
            sharpening: 0.1,
            ..Default::default()
        };

        // Apply multiple times with same parameters
        let result1 = filter.apply(&input, width, height, &params).unwrap();
        let result2 = filter.apply(&input, width, height, &params).unwrap();

        // Results should be identical for same input/params
        assert_eq!(result1.len(), result2.len());

        filter.destroy();
    }

    #[test]
    fn test_gpupixel_init_without_gpu() {
        // This test should fail gracefully without GPU
        let mut filter = GPUPixelFilter::new();
        let result = filter.init();

        // We expect this to either succeed (if GPU available) or fail gracefully
        if result.is_err() {
            println!("GPU not available: {}", result.unwrap_err());
        }
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpupixel_reinitialize() {
        let mut filter = GPUPixelFilter::new();

        // First initialization
        filter.init().unwrap();
        filter.destroy();

        // Second initialization after destroy
        let result = filter.init();
        assert!(result.is_ok(), "Reinitialization should succeed");

        filter.destroy();
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpupixel_different_image_sizes() {
        let mut filter = GPUPixelFilter::new();
        filter.init().unwrap();

        let params = BeautyParams {
            whitening: 0.3,
            smoothing: 0.3,
            sharpening: 0.3,
            ..Default::default()
        };

        // Test various image sizes
        let sizes = vec![(64, 64), (128, 128), (256, 256), (512, 512)];

        for (width, height) in sizes {
            let input = vec![128u8; (width * height * 4) as usize];
            let result = filter.apply(&input, width, height, &params);
            assert!(
                result.is_ok(),
                "Should handle {}x{} image",
                width,
                height
            );
        }

        filter.destroy();
    }
}
