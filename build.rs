// use vergen::{Config, vergen, SemverKind, ShaKind};

use std::fs::File;

use rustc_version::{version_meta, VersionMeta};
use git_version::git_version;

struct VersionInfo {
    rustc: VersionMeta,
    git: &'static str
}

const GIT_VERSION: &str = git_version!(prefix = "git:", cargo_prefix = "cargo:", fallback = "unknown");

fn main() {
    let info = VersionInfo {
        rustc: version_meta().unwrap(),
        git: git_version!()
    };
    // let mut config = Config::default();
    // *config.git_mut().sha_kind_mut() = ShaKind::Short;
    // *config.git_mut().semver_kind_mut() = SemverKind::Lightweight;
    // *config.git_mut().semver_dirty_mut() = Some("-dirty");

    // vergen(config).unwrap();

    // println!("git semver: {}", env!("VERGEN_GIT_SEMVER"));
}
