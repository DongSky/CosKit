use std::os::raw::{c_float, c_int};

#[repr(C)]
pub struct GPBeautyPipeline {
    _private: [u8; 0],
}

extern "C" {
    /// Create a full pipeline (source -> beauty filter -> sink). NULL on failure.
    pub fn gp_beauty_pipeline_create() -> *mut GPBeautyPipeline;
    pub fn gp_beauty_pipeline_destroy(p: *mut GPBeautyPipeline);

    /// Beauty parameters (0.0 - 1.0)
    pub fn gp_beauty_set_white(p: *mut GPBeautyPipeline, value: c_float);
    pub fn gp_beauty_set_blur_alpha(p: *mut GPBeautyPipeline, value: c_float);
    pub fn gp_beauty_set_sharpen(p: *mut GPBeautyPipeline, value: c_float);

    /// Process one RGBA frame. Returns 0 on success.
    pub fn gp_beauty_process_rgba(
        p: *mut GPBeautyPipeline,
        rgba: *const u8,
        width: c_int,
        height: c_int,
        stride_bytes: c_int,
    ) -> c_int;

    /// Result of the last process call; valid until the next process call.
    pub fn gp_beauty_get_rgba(
        p: *mut GPBeautyPipeline,
        out_width: *mut c_int,
        out_height: *mut c_int,
    ) -> *const u8;
}
