fn main() {
    // gpupixel.framework is a dylib; test/bin targets need an rpath pointing
    // at the vendored copy (dependency build scripts can't add link args to
    // the final binary, so it must happen here). Features are exposed to
    // build scripts as CARGO_FEATURE_* env vars.
    if std::env::var_os("CARGO_FEATURE_GPUPIXEL").is_some() {
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let fw_dir = manifest_dir.join("vendor/gpupixel-sys/prebuilt/mac");
        if fw_dir.exists() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", fw_dir.display());
        }
    }

    tauri_build::build()
}
