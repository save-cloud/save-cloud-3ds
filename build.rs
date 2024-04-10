use std::{env, fs, path::Path, process::Command};

use cc::Build;

fn build_image() {
    let path = Path::new("./romfs/assets.t3x");
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    let res = Command::new("tex3ds")
        .arg("-i")
        .arg("./assets/assets.t3s")
        .arg("-o")
        .arg("./romfs/assets.t3x")
        .output()
        .expect("failed to generate assets.t3x");

    if !res.status.success() {
        panic!("failed to generate assets.t3x");
    }
}

fn _copy_font() {
    let path = Path::new("./romfs/font.bcfnt");
    if path.exists() {
        return;
    }
    fs::copy("./assets/font.bcfnt", path).expect("copy font.bcfnt");
}

fn cc_build() -> Build {
    let devkitarm = env::var("DEVKITARM").unwrap();
    let cc = Path::new(devkitarm.as_str()).join("bin/arm-none-eabi-gcc");
    let ar = Path::new(devkitarm.as_str()).join("bin/arm-none-eabi-ar");

    let mut build = cc::Build::new();
    build
        .target("armv6k-nintendo-3ds")
        .compiler(&cc)
        .archiver(&ar)
        .include("/opt/devkitpro/devkitARM/arm-none-eabi/include")
        .include("/opt/devkitpro/libctru/include")
        .include("/opt/devkitpro/portlibs/3ds/include")
        .file("./c/http.c")
        .flag("-march=armv6k")
        .flag("-mtune=mpcore")
        .flag("-mfloat-abi=hard")
        .flag("-mfpu=vfp")
        .flag("-mtp=soft")
        .flag("-Wno-deprecated-declarations");
    build
}

fn main() {
    build_image();
    // copy_font();

    println!("cargo:rerun-if-changed=./build.rs");
    println!("cargo:rerun-if-changed=./http.c");
    println!("cargo:rerun-if-changed=./assets");
    println!("cargo:rerun-if-changed=./romfs");
    println!("cargo:rustc-link-search=all=./c");
    println!("cargo:rustc-link-search=all=/opt/devkitpro/portlibs/3ds/lib");
    // curl
    println!("cargo:rustc-link-lib=static=curl");
    println!("cargo:rustc-link-lib=static=mbedtls");
    println!("cargo:rustc-link-lib=static=mbedcrypto");
    println!("cargo:rustc-link-lib=static=mbedx509");
    println!("cargo:rustc-link-lib=static=minizip");
    println!("cargo:rustc-link-lib=static=z");
    // citro2d
    println!("cargo:rustc-link-lib=static=citro2d");
    println!("cargo:rustc-link-lib=static=citro3d");

    // http
    cc_build().file("./c/http.c").compile("libhttp.a");
    // c2d
    cc_build().file("./c/c2d.c").compile("libc2d.a");
    // platform
    cc_build().file("./c/platform.c").compile("libplatform.a");
    // util
    cc_build().file("./c/util.c").compile("libutil.a");
    // loader
    cc_build().file("./c/loader.c").compile("libloader.a");
}
