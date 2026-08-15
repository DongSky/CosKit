/// Beauty filter abstraction layer
///
/// Provides a unified interface for multiple beauty filter providers, plus a
/// dedicated worker thread that owns all GL-bound filter state. OpenGL
/// contexts are thread-affine, so every filter call — for either provider —
/// is funneled through that single long-lived thread (this also serializes
/// GPUPixel and PixelFree calls, matching the verified test setup).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::OnceLock;

/// Unified beauty filter interface
pub trait BeautyFilter: Send + Sync {
    /// Initialize the filter (creates OpenGL context, loads resources)
    fn init(&mut self) -> Result<(), String>;

    /// Apply beauty effect to an RGBA image, returning RGBA bytes
    fn apply(
        &mut self,
        input: &[u8],
        width: u32,
        height: u32,
        params: &BeautyParams,
    ) -> Result<Vec<u8>, String>;

    /// Release resources
    fn destroy(&mut self);
}

/// Beauty parameters (normalized 0.0-1.0 range)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BeautyParams {
    /// Whitening strength (0.0 = no effect, 1.0 = maximum)
    #[serde(default)]
    pub whitening: f32,

    /// Skin smoothing strength (0.0 = no effect, 1.0 = maximum)
    #[serde(default)]
    pub smoothing: f32,

    /// Sharpening strength (0.0 = no effect, 1.0 = maximum)
    #[serde(default)]
    pub sharpening: f32,

    /// Ruddy (rosy) tone (0.0 = no effect, 1.0 = maximum)
    /// Only available for PixelFree provider
    #[serde(default)]
    pub ruddy: f32,

    /// Eye brightening (0.0 = no effect, 1.0 = maximum)
    /// Only available for PixelFree provider
    #[serde(default)]
    pub eye_brighten: f32,

    /// Face reshaping parameters (optional, PixelFree only)
    #[serde(default)]
    pub face_reshape: Option<FaceReshapeParams>,
}

/// Face reshaping parameters (PixelFree only)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FaceReshapeParams {
    /// Eye enlargement (0.0-1.0)
    #[serde(default)]
    pub eye_strength: f32,

    /// Face slimming (0.0-1.0)
    #[serde(default)]
    pub face_thinning: f32,

    /// V-shaped face (0.0-1.0)
    #[serde(default)]
    pub face_v: f32,

    /// Nose slimming (0.0-1.0)
    #[serde(default)]
    pub nose: f32,
}

/// Create a beauty filter instance for the specified provider
pub fn create_beauty_filter(provider: &str) -> Result<Box<dyn BeautyFilter>, String> {
    match provider {
        "gpupixel" => {
            #[cfg(feature = "gpupixel")]
            {
                Ok(Box::new(crate::beauty_gpupixel::GPUPixelFilter::new()))
            }
            #[cfg(not(feature = "gpupixel"))]
            {
                Err("此构建未启用 GPUPixel（需 --features gpupixel）".to_string())
            }
        }
        "pixelfree" => {
            #[cfg(feature = "pixelfree")]
            {
                Ok(Box::new(crate::beauty_pixelfree::PixelFreeFilter::new()))
            }
            #[cfg(not(feature = "pixelfree"))]
            {
                Err("此构建未启用 PixelFree（需 --features pixelfree）".to_string())
            }
        }
        _ => Err(format!("Unknown beauty provider: {}", provider)),
    }
}

/// Providers compiled into this build.
pub fn available_providers() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(feature = "gpupixel") {
        v.push("gpupixel");
    }
    if cfg!(feature = "pixelfree") {
        v.push("pixelfree");
    }
    v
}

// ── GL worker thread ─────────────────────────────────────────────────────────

struct BeautyJob {
    provider: String,
    params: BeautyParams,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    reply: mpsc::Sender<Result<Vec<u8>, String>>,
}

static WORKER: OnceLock<mpsc::Sender<BeautyJob>> = OnceLock::new();

fn worker_sender() -> &'static mpsc::Sender<BeautyJob> {
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<BeautyJob>();
        std::thread::Builder::new()
            .name("beauty-gl-worker".into())
            .spawn(move || {
                // Filter instances live (and are only touched) on this thread.
                let mut filters: HashMap<String, Box<dyn BeautyFilter>> = HashMap::new();
                while let Ok(job) = rx.recv() {
                    let result = (|| {
                        if !filters.contains_key(&job.provider) {
                            let mut f = create_beauty_filter(&job.provider)?;
                            f.init()?;
                            filters.insert(job.provider.clone(), f);
                        }
                        let f = filters.get_mut(&job.provider).unwrap();
                        f.apply(&job.rgba, job.width, job.height, &job.params)
                    })();
                    // Failed init must not leave a broken cached instance.
                    if result.is_err() {
                        if let Some(mut f) = filters.remove(&job.provider) {
                            f.destroy();
                        }
                    }
                    let _ = job.reply.send(result);
                }
            })
            .expect("spawn beauty worker");
        tx
    })
}

/// Run a beauty filter on the shared GL worker thread (blocking).
/// Call from `tokio::task::spawn_blocking` in async contexts.
pub fn process_blocking(
    provider: &str,
    params: &BeautyParams,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    worker_sender()
        .send(BeautyJob {
            provider: provider.to_string(),
            params: params.clone(),
            rgba,
            width,
            height,
            reply: reply_tx,
        })
        .map_err(|_| "beauty worker unavailable".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "beauty worker dropped reply".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_deserialize_with_defaults() {
        let p: BeautyParams = serde_json::from_str("{}").unwrap();
        assert_eq!(p.whitening, 0.0);
        assert!(p.face_reshape.is_none());

        let p: BeautyParams =
            serde_json::from_str(r#"{"whitening":0.5,"face_reshape":{"face_thinning":0.3}}"#)
                .unwrap();
        assert_eq!(p.whitening, 0.5);
        assert_eq!(p.face_reshape.unwrap().face_thinning, 0.3);
    }

    #[test]
    fn unknown_provider_rejected() {
        assert!(create_beauty_filter("nope").is_err());
    }

    #[test]
    fn available_providers_matches_features() {
        let v = available_providers();
        assert_eq!(v.contains(&"gpupixel"), cfg!(feature = "gpupixel"));
        assert_eq!(v.contains(&"pixelfree"), cfg!(feature = "pixelfree"));
    }
}
