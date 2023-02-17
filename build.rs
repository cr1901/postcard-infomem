use std::error::Error;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    fs::write(
        &PathBuf::from(env::var("OUT_DIR")?).join("version.rs"),
        format!("use crate::InfoStr;
        \n\
        const PIM_VERSION_MAJOR: usize = {};\n\
        const PIM_VERSION_MINOR: usize = {};\n\
        const PIM_VERSION_PATCH: usize = {};\n\
        const PIM_VERSION_PRE: Option<InfoStr<'static>> = {};\n\
        ",

        env::var("CARGO_PKG_VERSION_MAJOR")?.parse::<usize>()?,
        env::var("CARGO_PKG_VERSION_MINOR")?.parse::<usize>()?,
        env::var("CARGO_PKG_VERSION_PATCH")?.parse::<usize>()?,

        {
            let pre_str = env::var("CARGO_PKG_VERSION_PRE")?;

            if pre_str.len() == 0 {
                format!("None")
            } else {
                format!("Some(InfoStr::Borrowed(\"{}\"))", pre_str)
            }
        }
    ))?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
