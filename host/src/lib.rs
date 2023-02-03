use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

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

pub fn generate_from_env<'a>() -> Result<InfoMem<'a>, Box<dyn Error>> {
    let mut im = InfoMem::default();

    // CARGO_PKG_VERSION hardcoded while compiling this crate.
    im.version = Version::parse(env!("CARGO_PKG_VERSION"))?;

    im.user.name = Some(env!("CARGO_PKG_NAME").into());
    // CARGO_PKG_VERSION comes from whatever is running this build script.
    im.user.version = Some(Version::parse(&std::env::var("CARGO_PKG_VERSION")?)?);

    // Similar in spirit to https://github.com/fusion-engineering/rust-git-version,
    // except done at runtime of a build-script, not compile-time of a crate.
    im.user.git = match Command::new("git")
        .args(["describe", "--always", "--dirty", "--tags"])
        .output()
    {
        Ok(o) if o.status.success() => Some(match String::from_utf8(o.stdout) {
            Ok(s) => s.into(),
            Err(_) => "unknown".into(),
        }),
        _ => Some("unknown".into()),
    };

    im.user.build_date = Some(OffsetDateTime::now_local()?);

    if let Ok(rv) = version_meta() {
        im.rustc.version = Some(rv.semver);
        im.rustc.llvm_version = rv.llvm_version.map(|l| Version::new(l.major, l.minor, 0));
        im.rustc.git = extract_short_git_string(rv.short_version_string).map(Into::into);
        im.rustc.host = Some(rv.host.into());
        im.rustc.channel = Some(rv.channel);
    }

    Ok(im)
}

pub struct WriterConfig {
    header: bool,
}

impl WriterConfig {
    pub fn set_header(mut self, op: bool) -> Self {
        self.header = op;
        self
    }
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self { header: true }
    }
}

pub fn write_info_to_file<P>(im: &InfoMem, path: P, cfg: WriterConfig) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
{
    let mut fp = File::create(path)?;

    let buf = if cfg.header {
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
        let im = generate_from_env().unwrap();

        let ser = to_stdvec(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn round_trip_borrowed() {
        let im = generate_from_env().unwrap();

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
