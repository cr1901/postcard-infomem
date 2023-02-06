use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use bitflags::bitflags;
use postcard::to_stdvec;
use postcard_infomem::{to_stdvec_magic, InfoMem};
use rustc_version::version_meta;
use semver::Version;
use time::OffsetDateTime;

// The short string will be fine. rustc_version is supposed to support
// returning this, but rustc -Vv seems to return full string.
fn extract_short_git_string(s: String) -> Option<String> {
    let short_git_begin = s.find('(')? + 1;
    let short_git_end = s[short_git_begin..].find(' ')?;
    Some(s[short_git_begin..short_git_begin + short_git_end].to_string())
}

bitflags! {
    struct EnvConfigFlags: u32 {
        const APP_NAME = 1;
        const APP_VERSION = 1 << 1;
        const APP_GIT = 1 << 2;
        const APP_DATE = 1 << 3;
        const RUSTC_VERSION = 1 << 4;
        const RUSTC_LLVM = 1 << 5;
        const RUSTC_GIT = 1 << 6;
        const RUSTC_HOST = 1 << 7;
        const RUSTC_CHANNEL = 1 << 8;
    }
}

pub struct EnvConfig(EnvConfigFlags);

impl Default for EnvConfig {
    fn default() -> Self {
        Self(EnvConfigFlags::all())
    }
}

impl EnvConfig {
    pub fn none() -> Self {
        Self(EnvConfigFlags::empty())
    }

    pub fn set_app_name(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_NAME, op);
        self
    }

    pub fn set_app_version(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_VERSION, op);
        self
    }

    pub fn set_app_git(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_GIT, op);
        self
    }

    pub fn set_app_date(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_DATE, op);
        self
    }

    pub fn set_rustc_version(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_VERSION, op);
        self
    }

    pub fn set_rustc_llvm(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_LLVM, op);
        self
    }

    pub fn set_rustc_git(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_GIT, op);
        self
    }

    pub fn set_rustc_host(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_HOST, op);
        self
    }

    pub fn set_rustc_channel(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_CHANNEL, op);
        self
    }
}

pub fn generate_from_env<'a>(cfg: EnvConfig) -> Result<InfoMem<'a>, Box<dyn Error>> {
    let mut im = InfoMem::default();

    if cfg.0.contains(EnvConfigFlags::APP_NAME) {
        im.app.name = Some(env::var("CARGO_PKG_NAME")?.into());
    }

    if cfg.0.contains(EnvConfigFlags::APP_VERSION) {
        // CARGO_PKG_VERSION comes from whatever is running this build script.
        im.app.version = Some(Version::parse(&env::var("CARGO_PKG_VERSION")?)?);
    }

    // Similar in spirit to https://github.com/fusion-engineering/rust-git-version,
    // except done at runtime of a build-script, not compile-time of a crate.
    if cfg.0.contains(EnvConfigFlags::APP_GIT) {
        im.app.git = match Command::new("git")
            .args(["describe", "--always", "--dirty", "--tags"])
            .output()
        {
            Ok(o) if o.status.success() => Some(match String::from_utf8(o.stdout) {
                Ok(s) => s.into(),
                Err(_) => "unknown".into(),
            }),
            _ => Some("unknown".into()),
        };
    }

    if cfg.0.contains(EnvConfigFlags::APP_DATE) {
        im.app.build_date = Some(OffsetDateTime::now_local()?);
    }

    if cfg.0.intersects(
        EnvConfigFlags::RUSTC_VERSION
            | EnvConfigFlags::RUSTC_LLVM
            | EnvConfigFlags::RUSTC_GIT
            | EnvConfigFlags::RUSTC_HOST
            | EnvConfigFlags::RUSTC_CHANNEL,
    ) {
        let rv = version_meta()?;

        if cfg.0.contains(EnvConfigFlags::RUSTC_VERSION) {
            im.rustc.version = Some(rv.semver);
        }

        if cfg.0.contains(EnvConfigFlags::RUSTC_LLVM) {
            im.rustc.llvm_version = rv.llvm_version.map(|l| Version::new(l.major, l.minor, 0));
        }

        if cfg.0.contains(EnvConfigFlags::RUSTC_GIT) {
            im.rustc.git = extract_short_git_string(rv.short_version_string).map(Into::into);
        }

        if cfg.0.contains(EnvConfigFlags::RUSTC_HOST) {
            im.rustc.host = Some(rv.host.into());
        }

        if cfg.0.contains(EnvConfigFlags::RUSTC_CHANNEL) {
            im.rustc.channel = Some(rv.channel);
        }
    }

    Ok(im)
}

bitflags! {
    struct WriterConfigFlags: u8 {
        const HEADER = 1;
    }
}

pub struct WriterConfig(WriterConfigFlags);

impl WriterConfig {
    pub fn set_header(mut self, op: bool) -> Self {
        self.0.set(WriterConfigFlags::HEADER, op);
        self
    }
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self(WriterConfigFlags::all())
    }
}

pub fn write_info_to_file<P>(im: &InfoMem, path: P, cfg: WriterConfig) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let mut fp = File::create(path)?;

    let buf = if cfg.0.contains(WriterConfigFlags::HEADER) {
        to_stdvec_magic(&im)?
    } else {
        to_stdvec(&im)?
    };

    fp.write_all(&buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postcard::from_bytes;

    #[test]
    fn round_trip_generate() {
        let im = generate_from_env(EnvConfig::default()).unwrap();

        let ser = to_stdvec(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn round_trip_borrowed() {
        let im = generate_from_env(EnvConfig::default()).unwrap();

        let ser = to_stdvec(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });

        fn borrow<'a>(stir: &'a [u8]) -> InfoMem<'a> {
            from_bytes(stir).unwrap()
        }

        let de = borrow(&ser);
        assert_eq!(im, de);
    }
}
