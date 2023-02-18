use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use postcard_infomem_host::*;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // This is default if no rerun-if-changed lines in build.rs.
    println!("cargo:rerun-if-changed=src");
    let im = generate_from_env(EnvConfig::default()).unwrap();
    write_info_to_file(&im, out.join("info.bin"), Default::default()).unwrap();

    do_linker_tasks_for_target(&out);
}

// Wrapper over the linker steps/args commonly set by applications for the
// given `target`.
fn do_linker_tasks_for_target(out: &Path) {
    let (arch, target) = decide_arch_target();

    match &*target {
        _ if env::var("CARGO_CFG_TARGET_OS").unwrap() != "none" && env::var("CARGO_CFG_TARGET_OS").unwrap() != "unknown" => {
            generate_infomem_ldscript(out.join("info.x"), HostedConfig::default()).unwrap();
        }
        "msp430g2553" if arch == "msp430" => {
            generate_infomem_ldscript(out.join("info.x"), BareSectionConfig::default()).unwrap();
            write_out_memory_x(&out, &target);
            println!("cargo:rustc-link-arg=-Tlink.x");
            println!("cargo:rustc-link-arg=-nostartfiles");
            println!("cargo:rustc-link-arg=-mcpu=msp430");
            println!("cargo:rustc-link-arg=-lmul_none");
            println!("cargo:rustc-link-arg=-lgcc");
        }
        "rp2040-hal" if arch == "arm" => {
            generate_infomem_ldscript(out.join("info.x"), BareAppendConfig::default()).unwrap();
            write_out_memory_x(&out, &target);
            println!("cargo:rustc-link-arg=-Tlink.x");
            println!("cargo:rustc-link-arg=--nmagic");
        },
        "ruduino" if arch == "avr" => {
            // No linker setup required for avr-gcc.
        },
        _ => unreachable!(),
    }
}

fn decide_arch_target() -> (String, String) {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Fast path for non-bare-metal stuff- ignore any features.
    if os != "none" && os != "unknown" {
        return (arch, os);
    }

    let targets = match &*arch {
        "msp430" => vec!["msp430g2553"],
        "arm" => vec!["rp2040-hal"],
        "avr" => vec!["ruduino"],
        s => unimplemented!("example is not implemented for arch {}", s),
    };

    // Look for first matching feature and return that as the target.
    match targets.iter().map_while(|t| {
        env::var(format!(
            "CARGO_FEATURE_{}",
            t.to_uppercase().replace('-', "_")
        )).ok().map(|_| (*t).to_owned())
    }).nth(0) {
        Some(t) => return (arch, t),
        None => panic!(
            "CARGO_CFG_TARGET_OS == \"none\", CARGO_CFG_TARGET_ARCH == \"{}\", one of the following features must be set: {}",
            arch,
            targets.join(",")
        )
    }
}

fn write_out_memory_x(out: &Path, target: &str) {
    // Find the appropriate memory script and copy to `out`.
    fs::copy(
        [
            env::var("CARGO_MANIFEST_DIR").unwrap(),
            "memory".into(),
            format!("{}.x", target),
        ]
        .iter()
        .collect::<PathBuf>(),
        out.join("memory.x"),
    )
    .unwrap();
    // Tell Rust where to find the memory script, rebuild when script changes.
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory/{}.x", target);
}
