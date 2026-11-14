fn main() {
    println!("cargo:rerun-if-changed=src/platform.m");
    println!("cargo:rerun-if-changed=src/platform.h");
    println!("cargo:rerun-if-changed=src/bookmarks.m");
    println!("cargo:rerun-if-changed=src/audio.m");
    println!("cargo:rerun-if-changed=src/scan.m");
    println!("cargo:rerun-if-changed=src/preferences.m");
    println!("cargo:rerun-if-changed=src/cloud.m");
    println!("cargo:rerun-if-changed=src/artwork.m");
    println!("cargo:rerun-if-changed=src/copy.m");
    println!("cargo:rerun-if-changed=src/profile.m");
    println!("cargo:rerun-if-changed=src/context.m");
    println!("cargo:rerun-if-changed=src/waveform.m");
    println!("cargo:rerun-if-changed=src/player.m");
    println!("cargo:rerun-if-changed=src/session.m");
    println!("cargo:rerun-if-changed=src/media.m");
    println!("cargo:rerun-if-changed=src/history.m");
    println!("cargo:rerun-if-changed=src/environment.m");
    println!("cargo:rerun-if-changed=src/window.m");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os == "macos" || os == "ios" {
        let mut native = cc::Build::new();
        if std::env::var_os("CARGO_FEATURE_INTEROP_TEST").is_some() {
            native.define("LW_INTEROP_TEST", None);
        }
        native
            .file("src/platform.m")
            .file("src/bookmarks.m")
            .file("src/audio.m")
            .file("src/scan.m")
            .file("src/preferences.m")
            .file("src/cloud.m")
            .file("src/artwork.m")
            .file("src/copy.m")
            .file("src/profile.m")
            .file("src/context.m")
            .file("src/waveform.m")
            .file("src/player.m")
            .file("src/session.m")
            .file("src/media.m")
            .file("src/history.m")
            .flag("-fobjc-arc")
            .warnings(true)
            .flag("-Werror");
        if os == "macos" {
            native
                .file("src/environment.m")
                .file("src/window.m")
                .flag("-mmacosx-version-min=15.0");
        } else {
            let target = std::env::var("TARGET").unwrap_or_default();
            native.flag(if target.ends_with("-sim") {
                "-mios-simulator-version-min=18.0"
            } else {
                "-miphoneos-version-min=18.0"
            });
        }
        native.compile("lw_apple");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!(
            "cargo:rustc-link-lib=framework={}",
            if os == "ios" { "UIKit" } else { "AppKit" }
        );
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=MediaPlayer");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=ImageIO");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
    }
}
