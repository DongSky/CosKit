use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo:rerun-if-changed=gl_context.c");
    println!("cargo:rerun-if-changed=tflite_stub.cpp");
    cc::Build::new()
        .file("gl_context.c")
        .define("GL_SILENCE_DEPRECATION", None)
        .compile("pf_gl_context");
    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++14")
        .file("tflite_stub.cpp")
        .compile("pf_tflite_stub");

    // Link pre-compiled PixelFree library
    match target_os.as_str() {
        "macos" => {
            println!(
                "cargo:rustc-link-search=native={}",
                manifest_dir.display()
            );
            println!("cargo:rustc-link-lib=static=PixelFree_mac");
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=framework=OpenGL");
            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-lib=framework=CoreGraphics");
            println!("cargo:rustc-link-lib=framework=CoreVideo");
            println!("cargo:rustc-link-lib=framework=Accelerate");
            // libPixelFree.a bundles libcurl objects; satisfy their deps.
            println!("cargo:rustc-link-lib=framework=SystemConfiguration");
            println!("cargo:rustc-link-lib=ldap");
            println!("cargo:rustc-link-lib=lber");
            for brew_pkg in ["libnghttp2", "libidn2"] {
                let p = format!("/opt/homebrew/opt/{brew_pkg}/lib");
                if std::path::Path::new(&p).exists() {
                    println!("cargo:rustc-link-search=native={p}");
                }
            }
            println!("cargo:rustc-link-lib=nghttp2");
            println!("cargo:rustc-link-lib=idn2");
        }
        "linux" => {
            println!(
                "cargo:rustc-link-search=native={}",
                manifest_dir.display()
            );
            println!("cargo:rustc-link-lib=static=PixelFree_linux");
            println!("cargo:rustc-link-lib=stdc++");
            println!("cargo:rustc-link-lib=GL");
        }
        "windows" => {
            println!(
                "cargo:rustc-link-search=native={}",
                manifest_dir.display()
            );
            println!("cargo:rustc-link-lib=static=PixelFree_windows");
            println!("cargo:rustc-link-lib=opengl32");
        }
        other => panic!("Unsupported platform for pixelfree-sys: {other}"),
    }

    println!("cargo:rerun-if-changed=libPixelFree_mac.a");
}
