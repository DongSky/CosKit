//! PixelFree runtime resource downloader.
//!
//! The PixelFree beauty backend needs two files at runtime that are NOT
//! shipped with CosKit (they are license-type artifacts from the vendor's
//! public demo): `pixelfreeAuth.lic` and `filter_model.bundle`.
//!
//! After the user accepts the in-app agreement, this module downloads both
//! files into `<data_dir>/pixelfree_res/`, trying each mirror in order and
//! verifying SHA-256 before the atomic rename into place. URLs are pinned
//! to a specific upstream commit so a future upstream change can never
//! silently swap the payload (the hash check would fail closed anyway).
//!
//! Mirror status (verified 2026-08-14, hashes matched on both):
//!   - raw.githubusercontent.com  — primary
//!   - cdn.jsdelivr.net           — backup, works in mainland China

use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::settings;

const REPO: &str = "uu-code007/PixelFreeEffects";
const COMMIT: &str = "156325c2a1a570683a63774b9ef623abdac25549";
const RES_PATH: &str = "SMBeautyEngine_mac/Res";

pub struct ResFile {
    pub name: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}

pub const RES_FILES: [ResFile; 2] = [
    ResFile {
        name: "pixelfreeAuth.lic",
        sha256: "a2adea67b107f21879395e389ef6768f07f0ba277014bf8f60ca180b28251dd5",
        size: 140_508,
    },
    ResFile {
        name: "filter_model.bundle",
        sha256: "dc91c7aded557066a1da6ed3ef591848da59c4d0b1850a52152ced5c1aabba91",
        size: 18_698_356,
    },
];

/// Directory the downloader installs into (also consulted by the
/// PixelFree provider's resource lookup).
pub fn res_dir() -> PathBuf {
    settings::data_dir().join("pixelfree_res")
}

fn mirror_urls(file: &str) -> [String; 2] {
    [
        format!("https://raw.githubusercontent.com/{REPO}/{COMMIT}/{RES_PATH}/{file}"),
        format!("https://cdn.jsdelivr.net/gh/{REPO}@{COMMIT}/{RES_PATH}/{file}"),
    ]
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

/// True when every resource file is present with the right size.
/// (Full hash verification happens at download time; size check here keeps
/// startup fast while still catching truncated files.)
pub fn resources_ready() -> bool {
    let dir = res_dir();
    RES_FILES.iter().all(|f| {
        std::fs::metadata(dir.join(f.name))
            .map(|m| m.len() == f.size)
            .unwrap_or(false)
    })
}

/// Download one file, trying mirrors in order. Returns the verified bytes.
async fn fetch_verified(file: &ResFile) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = String::new();
    for url in mirror_urls(file.name) {
        eprintln!("[CosKit] pixelfree_res: fetching {} from {url}", file.name);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(bytes) => {
                    let actual = sha256_hex(&bytes);
                    if actual == file.sha256 {
                        return Ok(bytes.to_vec());
                    }
                    last_err = format!(
                        "{url}: SHA-256 mismatch (got {actual}, expected {})",
                        file.sha256
                    );
                    eprintln!("[CosKit] pixelfree_res: {last_err}");
                }
                Err(e) => {
                    last_err = format!("{url}: body read failed: {e}");
                    eprintln!("[CosKit] pixelfree_res: {last_err}");
                }
            },
            Ok(resp) => {
                last_err = format!("{url}: HTTP {}", resp.status());
                eprintln!("[CosKit] pixelfree_res: {last_err}");
            }
            Err(e) => {
                last_err = format!("{url}: request failed: {e}");
                eprintln!("[CosKit] pixelfree_res: {last_err}");
            }
        }
    }
    Err(format!(
        "所有镜像均下载失败（{}）。最后错误：{last_err}",
        file.name
    ))
}

/// Download all resource files into `res_dir()`, emitting coarse progress
/// through the callback: (file_name, files_done, files_total, phase).
pub async fn download_all<F: Fn(&str, usize, usize, &str)>(progress: F) -> Result<(), String> {
    let dir = res_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败 {}: {e}", dir.display()))?;

    let total = RES_FILES.len();
    for (i, f) in RES_FILES.iter().enumerate() {
        let target = dir.join(f.name);

        // Skip files that are already present and intact.
        if let Ok(existing) = std::fs::read(&target) {
            if sha256_hex(&existing) == f.sha256 {
                progress(f.name, i + 1, total, "cached");
                continue;
            }
        }

        progress(f.name, i, total, "downloading");
        let bytes = fetch_verified(f).await?;

        progress(f.name, i, total, "writing");
        let tmp = dir.join(format!("{}.part", f.name));
        std::fs::write(&tmp, &bytes).map_err(|e| format!("写入失败 {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &target)
            .map_err(|e| format!("重命名失败 {}: {e}", target.display()))?;

        progress(f.name, i + 1, total, "done");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_well_formed() {
        for f in &RES_FILES {
            assert_eq!(f.sha256.len(), 64, "{} hash length", f.name);
            assert!(
                f.sha256.chars().all(|c| c.is_ascii_hexdigit()),
                "{} hash hex",
                f.name
            );
            assert!(f.size > 0);
        }
    }

    #[test]
    fn mirror_urls_are_pinned_to_commit() {
        for f in &RES_FILES {
            for url in mirror_urls(f.name) {
                assert!(url.contains(COMMIT), "URL not pinned: {url}");
                assert!(url.starts_with("https://"), "URL not https: {url}");
            }
        }
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // SHA-256("abc")
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[tokio::test]
    #[ignore] // network
    async fn download_all_works() {
        let tmp = std::env::temp_dir().join("coskit_pfres_test");
        let _ = std::fs::remove_dir_all(&tmp);
        // Point data_dir at a temp location via the settings override is not
        // trivial here; instead just fetch and verify the small file.
        let bytes = fetch_verified(&RES_FILES[0]).await.expect("fetch lic");
        assert_eq!(bytes.len() as u64, RES_FILES[0].size);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
