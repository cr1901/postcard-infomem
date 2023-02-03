use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use postcard_infomem_host::*;

fn main() {
    // Right now, embedding infomem into a hosted app is unsupported.
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "none" {
        return;
    } else {
        let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

        let im = generate_from_env().unwrap();
        write_info_to_file(&im, out.join("info.bin"), Default::default()).unwrap();

        let (arch, target) = decide_arch_target();
        write_out_memory_x(&out, &target);
        decide_link_args(&arch, &target);
    }
}

fn decide_arch_target() -> (String, String) {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
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

fn write_out_memory_x(out: &Path, target: &str) {
    // Copy `memory.x` to OUT_DIR.

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
    println!("cargo:rustc-link-search={}", out.display());

    // Rebuild when `memory.x` changes.
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
        _ => unreachable!(),
    }
}
