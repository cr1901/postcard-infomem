use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use postcard_infomem_host::*;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // This is default if no rerun-if-changed lines in build.rs.
    println!("cargo:rerun-if-changed=src");
    let im = generate_from_env(EnvConfig::default()).unwrap();
    write_info_to_file(&im, out.join("info.bin"), Default::default()).unwrap();

    let (arch, target) = decide_arch_target();

    generate_ldscript_for_target(&out, &arch, &target).unwrap();
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "none" {
        write_out_memory_x(&out, &target);
        decide_link_args(&arch, &target);
    }
}

fn decide_arch_target() -> (String, String) {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Fast path for non-bare-metal stuff- ignore any features.
    if os != "none" {
        return (arch, os);
    }

    let targets = match &*arch {
        "msp430" => vec!["msp430g2553"],
        "arm" => vec!["rp2040-hal"],
        s => unimplemented!("example is not implemented for arch {}", s),
    };

    for t in targets.clone() {
        if env::var("CARGO_FEATURE_".to_owned() + &t.to_uppercase().replace('-', "_")).is_ok() {
            return (arch, t.to_owned());
        }
    }

    panic!(
        "CARGO_CFG_TARGET_OS == \"none\", one of the following features must be set: {}",
        targets.join(",")
    )
}

fn generate_ldscript_for_target(out: &Path, arch: &str, target: &str) -> Result<(), Box<dyn Error>> {
    match target {
        _ if env::var("CARGO_CFG_TARGET_OS").unwrap() != "none" => {
            generate_infomem_ldscript(out.join("info.x"), HostedConfig::default())
        },
        "msp430g2553" if arch == "msp430" => {
            generate_infomem_ldscript(out.join("info.x"), BareSectionConfig::default())
        }
        "rp2040-hal" if arch == "arm" => {
            generate_infomem_ldscript(out.join("info.x"), BareAppendConfig::default())
        },
        _ => unreachable!(),
    }
}

fn write_out_memory_x(out: &Path, target: &str) {
    // Find the appropriate linker script and copy to `out`.
    let memory_x_path: PathBuf = [
        &*env::var("CARGO_MANIFEST_DIR").unwrap(),
        &"memory",
        &[target, ".x"].join(""),
    ]
    .iter()
    .collect();

    let mut inp_memory_x = Vec::new();
    File::open(memory_x_path)
        .unwrap()
        .read_to_end(&mut inp_memory_x)
        .unwrap();

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(&inp_memory_x)
        .unwrap();

    // Tell Rust where to find the linker script, rebuild when script changes.
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory/{}.x", target);
}

fn decide_link_args(arch: &str, target: &str) {
    match target {
        "msp430g2553" if arch == "msp430" => {
            println!("cargo:rustc-link-arg=-Tlink.x");
            println!("cargo:rustc-link-arg=-nostartfiles");
            println!("cargo:rustc-link-arg=-mcpu=msp430");
            println!("cargo:rustc-link-arg=-lmul_none");
            println!("cargo:rustc-link-arg=-lgcc");
        }
        "rp2040-hal" if arch == "arm" => {
            println!("cargo:rustc-link-arg=-Tlink.x");
            println!("cargo:rustc-link-arg=--nmagic");
        },
        _ => unreachable!(),
    }
}
