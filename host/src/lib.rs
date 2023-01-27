use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use postcard::to_stdvec;
use postcard_infomem::InfoMem;
use rustc_version::version_meta;
use semver;
use time::OffsetDateTime;

// The short string will be fine. rustc_version is supposed to support
// returning this, but rustc -Vv seems to return full string.
fn extract_short_git_string(s: String) -> Option<String> {
    let short_git_begin = s.find('(')? + 1;
    let short_git_end = s[short_git_begin..].find(' ')?;
    Some(s[short_git_begin..short_git_begin+short_git_end].to_string())
}

pub fn generate_from_env() -> Result<InfoMem, Box<dyn Error>> {
    let mut im = InfoMem::default();

    // CARGO_PKG_VERSION hardcoded while compiling this crate.
    im.version = Some(semver::Version::parse(env!("CARGO_PKG_VERSION"))?);

    // CARGO_PKG_VERSION comes from whatever is running this build script.
    im.user.version =
        Some(semver::Version::parse(&std::env::var("CARGO_PKG_VERSION")?)?);
    
    // Similar in spirit to https://github.com/fusion-engineering/rust-git-version,
    // except done at runtime of a build-script, not compile-time of a crate.
    im.user.git = match Command::new("git").args(["describe", "--always", "--dirty"]).output() {
        Ok(o) if o.status.success() => Some(String::from_utf8(o.stdout).unwrap_or("unknown".to_string())),
        _ => Some("unknown".to_string()),
    };

    im.user.build_date = Some(OffsetDateTime::now_local()?);

    if let Ok(rv) = version_meta() {
        im.rustc.version = Some(rv.semver);
        im.rustc.llvm_version = rv
            .llvm_version
            .map(|l| semver::Version::new(l.major, l.minor, 0));
        im.rustc.git = extract_short_git_string(rv.short_version_string);
        im.rustc.host = Some(rv.host);
        im.rustc.channel = Some(rv.channel);
    }

    Ok(im)
}

pub fn write_info_to_file<P>(im: &InfoMem, path: P) -> Result<(), Box<dyn Error>> where P: AsRef<Path> {
    let mut fp = File::create(path)?;
    let buf = to_stdvec(&im)?;
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
}
