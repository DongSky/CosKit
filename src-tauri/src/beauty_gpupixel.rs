/// GPUPixel beauty filter implementation
///
/// Wraps the gpupixel-sys pipeline (SourceRawData -> BeautyFaceFilter ->
/// SinkRawData) behind the unified BeautyFilter trait.
use crate::beauty_filter::{BeautyFilter, BeautyParams};
use std::ptr;

pub struct GPUPixelFilter {
    pipeline: *mut gpupixel_sys::GPBeautyPipeline,
}

impl GPUPixelFilter {
    pub fn new() -> Self {
        Self {
            pipeline: ptr::null_mut(),
        }
    }
}

impl Default for GPUPixelFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl BeautyFilter for GPUPixelFilter {
    fn init(&mut self) -> Result<(), String> {
        if !self.pipeline.is_null() {
            return Ok(());
        }
        unsafe {
            self.pipeline = gpupixel_sys::gp_beauty_pipeline_create();
        }
        if self.pipeline.is_null() {
            return Err("Failed to create GPUPixel pipeline (no GL context?)".to_string());
        }
        Ok(())
    }

    fn apply(
        &mut self,
        input: &[u8],
        width: u32,
        height: u32,
        params: &BeautyParams,
    ) -> Result<Vec<u8>, String> {
        if self.pipeline.is_null() {
            return Err("Filter not initialized".to_string());
        }
        let expected = (width as usize) * (height as usize) * 4;
        if input.len() < expected {
            return Err(format!(
                "Input buffer too small: {} < {} (RGBA {}x{})",
                input.len(),
                expected,
                width,
                height
            ));
        }

        unsafe {
            gpupixel_sys::gp_beauty_set_white(self.pipeline, params.whitening.clamp(0.0, 1.0));
            gpupixel_sys::gp_beauty_set_blur_alpha(self.pipeline, params.smoothing.clamp(0.0, 1.0));
            gpupixel_sys::gp_beauty_set_sharpen(self.pipeline, params.sharpening.clamp(0.0, 1.0));

            let rc = gpupixel_sys::gp_beauty_process_rgba(
                self.pipeline,
                input.as_ptr(),
                width as i32,
                height as i32,
                (width * 4) as i32,
            );
            if rc != 0 {
                return Err(format!("GPUPixel processing failed (code {rc})"));
            }

            let mut out_w: i32 = 0;
            let mut out_h: i32 = 0;
            let data = gpupixel_sys::gp_beauty_get_rgba(self.pipeline, &mut out_w, &mut out_h);
            if data.is_null() || out_w <= 0 || out_h <= 0 {
                return Err("GPUPixel returned no output".to_string());
            }
            let out_len = (out_w as usize) * (out_h as usize) * 4;
            Ok(std::slice::from_raw_parts(data, out_len).to_vec())
        }
    }

    fn destroy(&mut self) {
        if !self.pipeline.is_null() {
            unsafe {
                gpupixel_sys::gp_beauty_pipeline_destroy(self.pipeline);
            }
            self.pipeline = ptr::null_mut();
        }
    }
}

impl Drop for GPUPixelFilter {
    fn drop(&mut self) {
        self.destroy();
    }
}

// The pipeline owns its own GL state; access is serialized through &mut self.
unsafe impl Send for GPUPixelFilter {}
unsafe impl Sync for GPUPixelFilter {}
