use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.cpp");
    println!("cargo:rerun-if-changed=wrapper.h");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .flag_if_supported("-std=c++14")
        .include(manifest_dir.join("prebuilt/include"))
        .file("wrapper.cpp");

    match target_os.as_str() {
        "macos" => {
            // SinkRawData header imports AVFoundation/Foundation on macOS
            // (gpupixel_define.h defines GPUPIXEL_MAC itself); compile the
            // wrapper as Objective-C++ so those imports parse.
            build.flag("-ObjC++");

            let fw_dir = manifest_dir.join("prebuilt/mac");
            println!("cargo:rustc-link-search=framework={}", fw_dir.display());
            println!("cargo:rustc-link-lib=framework=gpupixel");
            println!("cargo:rustc-link-lib=framework=OpenGL");
            println!("cargo:rustc-link-lib=framework=AVFoundation");
            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=CoreVideo");
            println!("cargo:rustc-link-lib=framework=CoreMedia");
            println!("cargo:rustc-link-lib=c++");
            // Embed an rpath so the dylib inside the framework is found at
            // runtime when placed next to the app binary or in the vendor dir.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", fw_dir.display());
        }
        "linux" => {
            build.define("GPUPIXEL_LINUX", None);
            let lib_dir = manifest_dir.join("prebuilt/linux/lib");
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=dylib=gpupixel");
            println!("cargo:rustc-link-lib=GL");
            println!("cargo:rustc-link-lib=stdc++");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        }
        "windows" => {
            build.define("GPUPIXEL_WIN", None);
            let lib_dir = manifest_dir.join("prebuilt/windows/lib");
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=dylib=gpupixel");
            println!("cargo:rustc-link-lib=opengl32");
        }
        other => panic!("Unsupported platform for gpupixel-sys: {other}"),
    }

    build.compile("gpupixel_wrapper");
}
