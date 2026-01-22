use std::{env, path::PathBuf};

use anyhow::Result;

#[path = "gen/builder.rs"]
mod builder;

fn main() -> Result<()> {
    let crate_root_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());

    builder::OpenThreadBuilder::track(&crate_root_path.join("gen"));
    builder::OpenThreadBuilder::track(&crate_root_path.join("openthread"));

    // If `custom` is enabled, we need to re-build the bindings on-the-fly even if there are
    // pre-generated bindings for the target triple

    let host = env::var("HOST").unwrap();
    let target = env::var("TARGET").unwrap();

    let force_esp_riscv_toolchain = env::var("CARGO_FEATURE_FORCE_ESP_RISCV_TOOLCHAIN").is_ok();
    let ftd = env::var("CARGO_FEATURE_FTD").is_ok();
    let mtd = env::var("CARGO_FEATURE_MTD").is_ok();

    // Determine which mode to use (FTD takes precedence if both are somehow enabled)
    let ftd = ftd && !mtd;

    let pregen_bindings = env::var("CARGO_FEATURE_FORCE_GENERATE_BINDINGS").is_err();
    let pregen_bindings_rs_file = crate_root_path
        .join("src")
        .join("include")
        .join(format!("{target}.rs"));
    let pregen_libs_dir = crate_root_path.join("libs").join(&target);

    let dirs = if pregen_bindings && pregen_bindings_rs_file.exists() && pregen_libs_dir.exists() {
        // Check if we have the right libraries (FTD vs MTD)
        let has_ftd_lib = pregen_libs_dir.join("libopenthread-ftd.a").exists();
        let has_mtd_lib = pregen_libs_dir.join("libopenthread-mtd.a").exists();

        let libs_match = if ftd { has_ftd_lib } else { has_mtd_lib };

        if libs_match {
            // Use the pre-generated bindings and libraries
            Some((pregen_bindings_rs_file, pregen_libs_dir))
        } else {
            // Libraries don't match what we need, fall through to on-the-fly build
            log::warn!(
                "Pre-built libraries don't match requested mode (FTD={ftd}), will build on-the-fly"
            );
            None
        }
    } else if target.ends_with("-espidf") {
        // Nothing to do for ESP-IDF, `esp-idf-sys` will do everything for us
        None
    } else {
        // Need to do on-the-fly build and bindings' generation
        let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());

        let builder = builder::OpenThreadBuilder::new(
            crate_root_path.clone(),
            Some(target),
            Some(host),
            None,
            None,
            None,
            force_esp_riscv_toolchain,
        );

        let libs_dir = builder.compile(&out, None, ftd)?;
        let bindings = builder.generate_bindings(&out, None, ftd)?;

        Some((bindings, libs_dir))
    };

    if let Some((bindings, libs_dir)) = dirs {
        println!(
            "cargo::rustc-env=OPENTHREAD_SYS_BINDINGS_FILE={}",
            bindings.display()
        );

        println!("cargo:rustc-link-search={}", libs_dir.display());

        for entry in std::fs::read_dir(libs_dir)? {
            let entry = entry?;

            let file_name = entry.file_name();
            let file_name = file_name.to_str().unwrap();
            if file_name.ends_with(".a") || file_name.to_ascii_lowercase().ends_with(".lib") {
                let lib_name = if file_name.ends_with(".a") {
                    file_name.trim_start_matches("lib").trim_end_matches(".a")
                } else {
                    file_name.trim_end_matches(".lib")
                };

                // Filter out libraries that don't match the selected mode (FTD vs MTD)
                let skip = if ftd {
                    // When building for FTD, skip MTD-specific libraries
                    lib_name.ends_with("-mtd")
                } else {
                    // When building for MTD (default), skip FTD-specific libraries
                    lib_name.ends_with("-ftd")
                };

                if !skip {
                    println!("cargo:rustc-link-lib=static={lib_name}");
                }
            }
        }
    }

    Ok(())
}
