// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Builds the Qt Widgets frontend in `qt/` with CMake.
//!
//! The Rust core is compiled first, into a separate target directory. Cargo holds
//! a lock on the target directory it is using, so reusing it here would deadlock.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn run(mut command: Command, what: &str) {
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("failed to start {what}: {err}"));
    assert!(status.success(), "{what} failed with {status}");
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest_dir.join("../..").canonicalize().unwrap();
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let profile = env::var("PROFILE").unwrap();
    let release = profile == "release";

    println!("cargo:rerun-if-changed=../../qt");
    println!("cargo:rerun-if-changed=../../assets/icon.png");
    println!("cargo:rerun-if-changed=../../crates/ffi/src");

    // 1. Build the Rust core library, in its own target directory.
    let ffi_target = out_dir.join("ffi-target");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut build_core = Command::new(cargo);
    build_core
        .current_dir(&root)
        .args(["build", "-p", "nexusfm-ffi", "--target-dir"])
        .arg(&ffi_target)
        .env_remove("CARGO_TARGET_DIR");
    if release {
        build_core.arg("--release");
    }
    run(build_core, "building nexusfm-ffi");

    let rust_lib = ffi_target.join(&profile).join("libnexusfm_ffi.a");

    // 2. Configure and build the Qt frontend against that library.
    let cmake_build = out_dir.join("cmake-build");
    let build_type = if release { "Release" } else { "Debug" };

    let mut configure = Command::new("cmake");
    configure
        .arg("-S")
        .arg(root.join("qt"))
        .arg("-B")
        .arg(&cmake_build)
        .arg(format!("-DCMAKE_BUILD_TYPE={build_type}"))
        .arg(format!("-DNFM_RUST_LIB={}", rust_lib.display()));
    run(configure, "configuring the Qt frontend with CMake");

    let mut build = Command::new("cmake");
    build.arg("--build").arg(&cmake_build).arg("-j");
    run(build, "building the Qt frontend with CMake");

    println!(
        "cargo:rustc-env=NFM_QT_BIN={}",
        cmake_build.join("nexusfm-qt").display()
    );
}
