#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

use std::os::raw::{c_char, c_float, c_int, c_void};

#[repr(C)]
pub struct PFPixelFree {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PFDetectFormat {
    PFFORMAT_UNKNOWN = 0,
    PFFORMAT_IMAGE_RGB = 1,
    PFFORMAT_IMAGE_BGR = 2,
    PFFORMAT_IMAGE_RGBA = 3,
    PFFORMAT_IMAGE_BGRA = 4,
    PFFORMAT_IMAGE_ARGB = 5,
    PFFORMAT_IMAGE_ABGR = 6,
    PFFORMAT_IMAGE_GRAY = 7,
    PFFORMAT_IMAGE_YUV_NV12 = 8,
    PFFORMAT_IMAGE_YUV_NV21 = 9,
    PFFORMAT_IMAGE_YUV_I420 = 10,
    PFFORMAT_IMAGE_TEXTURE = 11,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PFRotationMode {
    PFRotationMode0 = 0,
    PFRotationMode90 = 1,
    PFRotationMode180 = 2,
    PFRotationMode270 = 3,
}

#[repr(C)]
pub struct PFImageInput {
    pub texture_id: c_int,
    pub width: c_int,
    pub height: c_int,
    pub p_data0: *mut c_void,
    pub p_data1: *mut c_void,
    pub p_data2: *mut c_void,
    pub stride_0: c_int,
    pub stride_1: c_int,
    pub stride_2: c_int,
    pub format: PFDetectFormat,
    pub rotation_mode: PFRotationMode,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PFBeautyFilterType {
    PFBeautyFilterTypeFace_EyeStrength = 0,
    PFBeautyFilterTypeFace_thinning = 1,
    PFBeautyFilterTypeFace_narrow = 2,
    PFBeautyFilterTypeFace_chin = 3,
    PFBeautyFilterTypeFace_V = 4,
    PFBeautyFilterTypeFace_small = 5,
    PFBeautyFilterTypeFace_nose = 6,
    PFBeautyFilterTypeFace_forehead = 7,
    PFBeautyFilterTypeFace_mouth = 8,
    PFBeautyFilterTypeFace_philtrum = 9,
    PFBeautyFilterTypeFace_long_nose = 10,
    PFBeautyFilterTypeFace_eye_space = 11,
    PFBeautyFilterTypeFace_smile = 12,
    PFBeautyFilterTypeFace_eye_rotate = 13,
    PFBeautyFilterTypeFace_canthus = 14,

    // Core beauty parameters
    PFBeautyFilterTypeFaceBlurStrength = 15,    // 磨皮 0-1
    PFBeautyFilterTypeFaceWhitenStrength = 16,  // 美白 0-1
    PFBeautyFilterTypeFaceRuddyStrength = 17,   // 红润 0-1
    PFBeautyFilterTypeFaceSharpenStrength = 18, // 锐化 0-1
    PFBeautyFilterTypeFaceEyeBrighten = 21,     // 亮眼 0-1
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PFSrcType {
    PFSrcTypeFilter = 0,
    PFSrcTypeAuthFile = 2,
    PFSrcTypeStickerFile = 3,
}

#[link(name = "PixelFree_mac", kind = "static")]
extern "C" {
    pub fn PF_Version() -> *const c_char;

    pub fn PF_NewPixelFree() -> *mut PFPixelFree;
    pub fn PF_DeletePixelFree(pixelFree: *mut PFPixelFree);

    pub fn PF_processWithBuffer(pixelFree: *mut PFPixelFree, inputImage: PFImageInput) -> c_int;

    pub fn PF_pixelFreeSetBeautyFilterParam(
        pixelFree: *mut PFPixelFree,
        key: c_int,
        value: *mut c_void,
    );

    /// Load an auth license / filter model bundle from memory.
    pub fn PF_createBeautyItemFormBundle(
        pixelFree: *mut PFPixelFree,
        data: *mut c_void,
        size: c_int,
        r#type: PFSrcType,
    );

    pub fn PF_pixelFreeGetFaceRect(pixelFree: *mut PFPixelFree, faceRect: *mut c_float);
    pub fn PF_pixelFreeHaveFaceSize(pixelFree: *mut PFPixelFree) -> c_int;
}

// Offscreen GL context helpers (gl_context.c). PixelFree needs a current
// OpenGL context on the calling thread before any PF_* call.
extern "C" {
    /// Create (or re-activate) the shared offscreen CGL context. 0 on success.
    pub fn pf_gl_init() -> c_int;
    /// Destroy the shared offscreen context.
    pub fn pf_gl_destroy();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixelfree_lifecycle() {
        unsafe {
            let pf = PF_NewPixelFree();
            assert!(!pf.is_null());
            PF_DeletePixelFree(pf);
        }
    }
}
