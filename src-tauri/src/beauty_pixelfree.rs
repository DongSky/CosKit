/// PixelFree beauty filter implementation
///
/// Wraps pixelfree-sys behind the unified BeautyFilter trait.
///
/// Init sequence (mirrors the vendor's SMBeautyEngine_mac demo):
///   1. Make an offscreen GL context current (pf_gl_init)
///   2. PF_NewPixelFree
///   3. Load auth license bundle (pixelfreeAuth.lic)
///   4. Load filter model bundle (filter_model.bundle)
///
/// Resource resolution order for the two bundles:
///   1. `PIXELFREE_RES_DIR` env var
///   2. `<data_dir>/pixelfree_res/` (installed by the in-app downloader)
///   3. `<crate>/vendor/pixelfree-sys/res/` (dev builds)
use crate::beauty_filter::{BeautyFilter, BeautyParams};
use std::os::raw::c_void;
use std::path::PathBuf;
use std::ptr;

pub struct PixelFreeFilter {
    handle: *mut pixelfree_sys::PFPixelFree,
    initialized: bool,
}

impl PixelFreeFilter {
    pub fn new() -> Self {
        Self {
            handle: ptr::null_mut(),
            initialized: false,
        }
    }

    fn res_dir() -> Result<PathBuf, String> {
        if let Ok(dir) = std::env::var("PIXELFREE_RES_DIR") {
            let p = PathBuf::from(dir);
            if p.join("pixelfreeAuth.lic").exists() {
                return Ok(p);
            }
        }
        let downloaded = crate::pixelfree_res::res_dir();
        if downloaded.join("pixelfreeAuth.lic").exists() {
            return Ok(downloaded);
        }
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/pixelfree-sys/res");
        if dev.join("pixelfreeAuth.lic").exists() {
            return Ok(dev);
        }
        Err("PixelFree 资源未就绪：请在设置中同意协议并下载资源，\
             或设置 PIXELFREE_RES_DIR 指向包含 pixelfreeAuth.lic 和 filter_model.bundle 的目录"
            .to_string())
    }

    unsafe fn load_bundle(
        &mut self,
        path: &std::path::Path,
        kind: pixelfree_sys::PFSrcType,
    ) -> Result<(), String> {
        let mut data =
            std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        pixelfree_sys::PF_createBeautyItemFormBundle(
            self.handle,
            data.as_mut_ptr() as *mut c_void,
            data.len() as i32,
            kind,
        );
        Ok(())
    }
}

impl Default for PixelFreeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl BeautyFilter for PixelFreeFilter {
    fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        let res_dir = Self::res_dir()?;

        unsafe {
            let rc = pixelfree_sys::pf_gl_init();
            if rc != 0 {
                return Err(format!("Failed to create offscreen GL context (code {rc})"));
            }

            self.handle = pixelfree_sys::PF_NewPixelFree();
            if self.handle.is_null() {
                return Err("Failed to create PixelFree instance".to_string());
            }

            self.load_bundle(
                &res_dir.join("pixelfreeAuth.lic"),
                pixelfree_sys::PFSrcType::PFSrcTypeAuthFile,
            )?;
            self.load_bundle(
                &res_dir.join("filter_model.bundle"),
                pixelfree_sys::PFSrcType::PFSrcTypeFilter,
            )?;

            self.initialized = true;
            Ok(())
        }
    }

    fn apply(
        &mut self,
        input: &[u8],
        width: u32,
        height: u32,
        params: &BeautyParams,
    ) -> Result<Vec<u8>, String> {
        if !self.initialized {
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

        // PixelFree processes in-place on the CPU buffer.
        let mut buffer = input.to_vec();

        unsafe {
            // GL context must be current on this thread for every call.
            let rc = pixelfree_sys::pf_gl_init();
            if rc != 0 {
                return Err(format!("Failed to activate GL context (code {rc})"));
            }

            self.set_param(
                pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFaceWhitenStrength,
                params.whitening.clamp(0.0, 1.0),
            );
            self.set_param(
                pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFaceBlurStrength,
                params.smoothing.clamp(0.0, 1.0),
            );
            self.set_param(
                pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFaceSharpenStrength,
                params.sharpening.clamp(0.0, 1.0),
            );
            self.set_param(
                pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFaceRuddyStrength,
                params.ruddy.clamp(0.0, 1.0),
            );
            self.set_param(
                pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFaceEyeBrighten,
                params.eye_brighten.clamp(0.0, 1.0),
            );

            if let Some(ref reshape) = params.face_reshape {
                self.set_param(
                    pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFace_EyeStrength,
                    reshape.eye_strength.clamp(0.0, 1.0),
                );
                self.set_param(
                    pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFace_thinning,
                    reshape.face_thinning.clamp(0.0, 1.0),
                );
                self.set_param(
                    pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFace_V,
                    reshape.face_v.clamp(0.0, 1.0),
                );
                self.set_param(
                    pixelfree_sys::PFBeautyFilterType::PFBeautyFilterTypeFace_nose,
                    reshape.nose.clamp(0.0, 1.0),
                );
            }

            let input_image = pixelfree_sys::PFImageInput {
                texture_id: 0,
                width: width as i32,
                height: height as i32,
                p_data0: buffer.as_mut_ptr() as *mut c_void,
                p_data1: ptr::null_mut(),
                p_data2: ptr::null_mut(),
                stride_0: (width * 4) as i32,
                stride_1: 0,
                stride_2: 0,
                format: pixelfree_sys::PFDetectFormat::PFFORMAT_IMAGE_RGBA,
                rotation_mode: pixelfree_sys::PFRotationMode::PFRotationMode0,
            };

            pixelfree_sys::PF_processWithBuffer(self.handle, input_image);
        }

        Ok(buffer)
    }

    fn destroy(&mut self) {
        if !self.initialized {
            return;
        }
        unsafe {
            if !self.handle.is_null() {
                pixelfree_sys::PF_DeletePixelFree(self.handle);
                self.handle = ptr::null_mut();
            }
        }
        self.initialized = false;
    }
}

impl PixelFreeFilter {
    unsafe fn set_param(&mut self, key: pixelfree_sys::PFBeautyFilterType, value: f32) {
        let mut v = value;
        let value_ptr = &mut v as *mut f32 as *mut c_void;
        pixelfree_sys::PF_pixelFreeSetBeautyFilterParam(self.handle, key as i32, value_ptr);
    }
}

impl Drop for PixelFreeFilter {
    fn drop(&mut self) {
        self.destroy();
    }
}

// Access is serialized through &mut self; the GL context is bound per-call.
unsafe impl Send for PixelFreeFilter {}
unsafe impl Sync for PixelFreeFilter {}
