/*! Helper crate for [`InfoMem`] `struct`s intended to primarily be used in
build scripts. */

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

mod ldscript;
pub use ldscript::{generate_infomem_ldscript};

// The short string will be fine. 
/** Workaround function to extract the short git SHA from `rustc -Vv`.

[`rustc_version`] is supposed to support returning a short hash, but does not
seem to postprocess `rustc -Vv`, which returns the full string. */
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

/// Flags for default arguments to [`generate_from_env`].
pub struct EnvConfig(EnvConfigFlags);

impl Default for EnvConfig {
    /// Populate all [`InfoMem`] fields.
    fn default() -> Self {
        Self(EnvConfigFlags::all())
    }
}

impl EnvConfig {
    /** Do not populate any fields of [`InfoMem`]. This function is intended
    to be used as a shortcut for enabling one or two flags with the remaining
    functions. */
    pub fn none() -> Self {
        Self(EnvConfigFlags::empty())
    }

    /// If `true`, set [`AppInfo::name`](postcard_infomem::AppInfo::name).
    pub fn set_app_name(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_NAME, op);
        self
    }

    /// If `true`, set [`AppInfo::version`](postcard_infomem::AppInfo::version).
    pub fn set_app_version(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_VERSION, op);
        self
    }

    /// If `true`, set [`AppInfo::git`](postcard_infomem::AppInfo::git).
    pub fn set_app_git(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_GIT, op);
        self
    }

    /// If `true`, set [`AppInfo::build_date`](postcard_infomem::AppInfo::build_date).
    pub fn set_app_date(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::APP_DATE, op);
        self
    }

    /// If `true`, set [`RustcInfo::version`](postcard_infomem::RustcInfo::version).
    pub fn set_rustc_version(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_VERSION, op);
        self
    }

    /// If `true`, set [`RustcInfo::llvm_version`](postcard_infomem::RustcInfo::llvm_version).
    pub fn set_rustc_llvm(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_LLVM, op);
        self
    }

    /// If `true`, set [`RustcInfo::git`](postcard_infomem::RustcInfo::git).
    pub fn set_rustc_git(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_GIT, op);
        self
    }

    /// If `true`, set [`RustcInfo::host`](postcard_infomem::RustcInfo::host).
    pub fn set_rustc_host(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_HOST, op);
        self
    }

    /// If `true`, set [`RustcInfo::channel`](postcard_infomem::RustcInfo::channel).
    pub fn set_rustc_channel(mut self, op: bool) -> Self {
        self.0.set(EnvConfigFlags::RUSTC_CHANNEL, op);
        self
    }
}

/** Populate an [`InfoMem`] struct using environment variables and host [`Command`]s.

For each flag enabled in [`EnvConfig`], [`generate_from_env`] attempts to set
one field of an [`InfoMem`] `struct`.

# How Fields Are Set
## [`app`](InfoMem::app):

* [`AppInfo::name`](postcard_infomem::AppInfo::name): Query the `CARGO_PKG_NAME` environment variable.
* [`AppInfo::version`](postcard_infomem::AppInfo::version): Query the `CARGO_PKG_VERSION` environment variable.
* [`AppInfo::git`](postcard_infomem::AppInfo::git): Run `git describe --always --dirty --tags` and
  capture the output. If this command fails to run (or fails to find a commit SHA),
  the value becomes `Some("unknown")`.
* [`AppInfo::build_date`](postcard_infomem::AppInfo::build_date): Use [`time`] to get the current _local_ time.

## [`rustc`](InfoMem::rustc)

All fields of [`rustc`](InfoMem::rustc) are populated from the return value of
[`version_meta`]. The [`RustcInfo::git`](postcard_infomem::RustcInfo::git)
field will return `Option::None` if extracting the `rustc` `git` SHA fails.

## [`user`](InfoMem::user)

_This function does not modify [`user`](InfoMem::user) from the [default](InfoMem::default)
value of `None`._ The user must populate this field through other means.

# Arguments
* `cfg`: Set of arguments that determine which fields of an [`InfoMem`]  that
  this function tries to set.

# Errors
All errors are casted to [`Box<dyn Error>`]. Concrete error types include:

* [`VarError`](env::VarError): Returned if an environment variable does not exist.
* [`semver::Error`]: Returned if any attempt to parse a [`Version`] fails.
* [`IndeterminateOffset`](time::error::IndeterminateOffset): Returned if getting the local time fails.
* [`rustc_version::Error`]: Returned if [`version_meta`] fails to run for any reason.

Notably _except for `git` fields_, [`generate_from_env`] will return an error
if it fails to populate _any_ field corresponding to the enabled flags in [`EnvConfig`].
*/
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

/// Flags for default arguments to [`generate_from_env`].
pub struct WriterConfig(WriterConfigFlags);

impl WriterConfig {
    /** If `true`, write out the [magic header](postcard_infomem::ser::Magic)
    before the serialized [`InfoMem`]. */
    pub fn set_header(mut self, op: bool) -> Self {
        self.0.set(WriterConfigFlags::HEADER, op);
        self
    }
}

impl Default for WriterConfig {
    /** By default, _enable_ writing the [magic header](postcard_infomem::ser::Magic)
    before the serialized [`InfoMem`]. */
    fn default() -> Self {
        Self(WriterConfigFlags::all())
    }
}

/** Write out a serialized [`InfoMem`] structure to file.

This is a convenience function intended to be used in a [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
The serialized [`InfoMem`] file can be embedded into an application by using the
[`include_postcard_infomem`](../postcard-infomem-device/macro.include_postcard_infomem.html)
macro.

# Arguments
* `im`: [`InfoMem`] `struct` to write out.
* `path`: Name of file to write to.
* `cfg`: Set of arguments that determine how to write out the serialized [`InfoMem`].

# Errors
All errors are casted to [`Box<dyn Error>`]. Concrete error types include:
* [`io::Error`](std::io::Error): Returned if creating or writing the file fails.
* [`postcard::Error`]: Returned if serializing `im` fails.

*/
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
