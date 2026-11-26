use std::path::PathBuf;
use std::process::Command;

fn main() {
    widget_bridge();
    tauri_build::build()
}

/// Compiles swift/WidgetBridge.swift into a static library and links it into the app.
/// WidgetKit has no C API, so the app reaches it through this Swift file.
fn widget_bridge() {
    const SOURCE: &str = "swift/WidgetBridge.swift";
    println!("cargo:rerun-if-changed={SOURCE}");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let lib = out.join("libwidgetbridge.a");
    // Swift names the Rust `aarch64` arch `arm64`. Other arches have the same name.
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH")
        .expect("target arch")
        .as_str()
    {
        "aarch64" => "arm64".to_string(),
        other => other.to_string(),
    };
    let target = format!("{arch}-apple-macos10.15");
    // The app runs from macOS 10.15. WidgetKit starts at macOS 11 and is linked weakly.
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-O",
            "-parse-as-library",
            "-emit-library",
            "-static",
        ])
        .args(["-module-name", "WidgetBridge"])
        .args(["-target", &target, SOURCE, "-o"])
        .arg(&lib)
        .status()
        .expect("run swiftc: install the Xcode command line tools");
    assert!(status.success(), "swiftc failed on {SOURCE}");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=widgetbridge");
    // The Swift runtime ships with macOS. The linker needs the SDK and toolchain stubs.
    let xcrun = |args: &[&str]| {
        let out = Command::new("xcrun")
            .args(args)
            .output()
            .expect("run xcrun");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let sdk = xcrun(&["--show-sdk-path"]);
    println!("cargo:rustc-link-search=native={sdk}/usr/lib/swift");
    if let Some(bin) = PathBuf::from(xcrun(&["-f", "swiftc"])).parent() {
        let toolchain = bin.join("../lib/swift/macosx");
        println!("cargo:rustc-link-search=native={}", toolchain.display());
    }
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
}
