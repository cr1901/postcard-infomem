use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use new_string_template::template::Template;

// { alignment } needs spaces between braces, otherwise template render fails...
static INFOMEM_LINKER_SCRIPT_TEMPLATE: &str = r#"
{header}

SECTIONS {
    { alignment }
    .info : {
        _sinfo = .;
        KEEP(*({info_section_name}))
        _einfo = .;
    } {memory_region}
} {insert_before_after}

{footer}
"#;

static INFOMEM_LINKER_SCRIPT_FOOTER_TEMPLATE: &str = r#"
ASSERT((_einfo - _sinfo) <= {max_safe_info_size}, "
ERROR({filename}): Information memory output section is greater than {max_safe_info_size} bytes long.
Flashing may overwrite important calibration data. The link has stopped as a precaution.
");
"#;

pub struct LdConfig<'a> {
    inp_section: &'a str,
    region: Option<&'a str>,
    insert: InsertType<'a>,
    max_size: Option<usize>,
    alignment: Option<&'a str>,
}

enum InsertType<'a> {
    None,
    #[allow(dead_code)]
    Before(&'a str),
    After(&'a str),
}

pub struct BareSectionConfig<'a> {
    inp_section: &'a str,
    region: &'a str,
    max_size: Option<usize>,
}

impl<'a> BareSectionConfig<'a> {
    pub fn set_info_section(mut self, sec: &'a str) -> Self {
        self.inp_section = sec;
        self
    }

    pub fn set_memory_region(mut self, reg: &'a str) -> Self {
        self.region = reg;
        self
    }

    pub fn set_max_size(mut self, size: Option<usize>) -> Self {
        self.max_size = size;
        self
    }
}

impl<'a> Default for BareSectionConfig<'a> {
    fn default() -> Self {
        Self {
            inp_section: ".info",
            region: "INFOMEM",
            max_size: None,
        }
    }
}

impl<'a> From<BareSectionConfig<'a>> for LdConfig<'a> {
    fn from(value: BareSectionConfig<'a>) -> Self {
        if cfg!(test)
            || env::var("CARGO_CFG_TARGET_OS").unwrap() == "none"
            || env::var("CARGO_CFG_TARGET_OS").unwrap() == "unknown"
        {
            LdConfig {
                inp_section: value.inp_section,
                region: Some(value.region),
                insert: InsertType::None,
                max_size: value.max_size,
                alignment: None,
            }
        } else {
            panic!("BareAppendConfig is only compatible with target_os = \"none\", current target_os = \"{}\"", env::var("CARGO_CFG_TARGET_OS").unwrap());
        }
    }
}

pub struct BareAppendConfig<'a> {
    inp_section: &'a str,
    out_section: &'a str,
    region: &'a str,
    max_size: Option<usize>,
}

impl<'a> BareAppendConfig<'a> {
    pub fn set_info_section(mut self, sec: &'a str) -> Self {
        self.inp_section = sec;
        self
    }

    pub fn set_append_section(mut self, sec: &'a str) -> Self {
        self.out_section = sec;
        self
    }

    pub fn set_memory_region(mut self, reg: &'a str) -> Self {
        self.region = reg;
        self
    }

    pub fn set_max_size(mut self, size: Option<usize>) -> Self {
        self.max_size = size;
        self
    }
}

impl<'a> Default for BareAppendConfig<'a> {
    fn default() -> Self {
        Self {
            inp_section: ".info",
            out_section: ".rodata",
            region: "FLASH",
            max_size: None,
        }
    }
}

impl<'a> From<BareAppendConfig<'a>> for LdConfig<'a> {
    fn from(value: BareAppendConfig<'a>) -> Self {
        if cfg!(test) || env::var("CARGO_CFG_TARGET_OS").unwrap() == "none" {
            LdConfig {
                inp_section: value.inp_section,
                region: Some(value.region),
                insert: InsertType::After(value.out_section),
                max_size: value.max_size,
                alignment: None,
            }
        } else {
            panic!("BareAppendConfig is only compatible with target_os = \"none\"");
        }
    }
}

pub struct HostedConfig<'a> {
    inp_section: &'a str,
}

impl<'a> Default for HostedConfig<'a> {
    fn default() -> Self {
        Self {
            inp_section: ".info",
        }
    }
}

impl<'a> From<HostedConfig<'a>> for LdConfig<'a> {
    fn from(value: HostedConfig<'a>) -> Self {
        if cfg!(test)
            || (env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows"
                && env::var("CARGO_CFG_TARGET_ENV").unwrap() == "gnu")
        {
            LdConfig {
                inp_section: value.inp_section,
                region: None,
                insert: InsertType::After(".text"),
                max_size: None,
                alignment: Some("__section_alignment__"),
            }
        // This will never be supported...
        } else if env::var("CARGO_CFG_TARGET_OS").unwrap() == "none" {
            panic!("HostedConfig is not compatible with target_os = \"none\"");
        // but some OSes that match this might be.
        } else {
            panic!(
                "HostedConfig is not compatible with target_os = {}, target_env = {}",
                env::var("CARGO_CFG_TARGET_OS").unwrap(),
                env::var("CARGO_CFG_TARGET_ENV").unwrap()
            );
        }
    }
}

pub fn generate_infomem_ldscript<'a, P, L>(path: P, cfg: L) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
    L: Into<LdConfig<'a>>,
{
    let filename = path
        .as_ref()
        .file_name()
        .ok_or("invalid filename for linker script")?
        .to_string_lossy();
    let dir = path
        .as_ref()
        .parent()
        .ok_or("invalid path for linker script")?
        .to_string_lossy();
    let script = generate_script(cfg.into(), &filename)?;
    let mut fp = File::create(&path)?;
    fp.write_all(&script.as_bytes())?;

    println!("cargo:rustc-link-arg=-T{}", filename);
    println!("cargo:rustc-link-search={}", dir);

    Ok(())
}

fn generate_script(cfg: LdConfig, filename: &str) -> Result<String, Box<dyn Error>> {
    let templ = Template::new(INFOMEM_LINKER_SCRIPT_TEMPLATE);

    let mut data: HashMap<&str, String> = HashMap::new();
    generate_header(&mut data, &cfg);
    generate_body(&mut data, &cfg);
    generate_footer(&mut data, &cfg, filename)?;

    Ok(templ.render(&data)?)
}

fn generate_header(data: &mut HashMap<&str, String>, _cfg: &LdConfig) {
    data.insert(
        "header",
        concat!(
            "/* Generated by ",
            env!("CARGO_PKG_NAME"),
            " version ",
            env!("CARGO_PKG_VERSION"),
            " */"
        )
        .into(),
    );
}

fn generate_body(data: &mut HashMap<&str, String>, cfg: &LdConfig) {
    data.insert("info_section_name", cfg.inp_section.into());

    match cfg.alignment {
        None => data.insert("alignment", "".into()),
        Some(s) => data.insert("alignment", format!(". = ALIGN({});", s)),
    };

    match cfg.region {
        None => data.insert("memory_region", "".into()),
        Some(s) => data.insert("memory_region", format!("> {}", s)),
    };

    match cfg.insert {
        InsertType::None => data.insert("insert_before_after", "".into()),
        InsertType::Before(s) => data.insert("insert_before_after", format!("INSERT BEFORE {}", s)),
        InsertType::After(s) => data.insert("insert_before_after", format!("INSERT AFTER {}", s)),
    };
}

fn generate_footer(
    data: &mut HashMap<&str, String>,
    cfg: &LdConfig,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    match cfg.max_size {
        None => {
            data.insert("footer", "".into());
        }
        Some(size) => {
            let footer_templ = Template::new(INFOMEM_LINKER_SCRIPT_FOOTER_TEMPLATE);

            let mut footer_data: HashMap<&str, String> = HashMap::new();
            footer_data.insert("max_safe_info_size", size.to_string());
            footer_data.insert("filename", filename.into());

            let footer = footer_templ.render(&footer_data)?;
            data.insert("footer", footer.into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use ldscript_parser as lds;

    fn assert_ldscript_eq(left: &str, right: &str) -> Result<(), String> {
        let ldl = lds::parse(left)?;
        let ldr = lds::parse(right)?;

        assert_eq!(ldl, ldr);

        Ok(())
    }

    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    #[test]
    fn generate_hosted_windows_gnu() {
        let cfg = HostedConfig::default().into();

        let lds = generate_script(cfg, "foo.x").unwrap();
        // FIXME: ldscript parser needs to be taught about "INSERT BEFORE/AFTER"...
        assert_eq!(
            &lds,
            indoc! {"
            
            /* Generated by postcard-infomem-host version 0.1.0 */
            
            SECTIONS {
                . = ALIGN(__section_alignment__);
                .info : {
                    _sinfo = .;
                    KEEP(*(.info))
                    _einfo = .;
                } 
            } INSERT AFTER .text
            
            
            "},
        );
    }

    #[test]
    fn generate_bare_append() {
        let cfg = BareAppendConfig::default().into();

        let lds = generate_script(cfg, "foo.x").unwrap();
        // FIXME: ldscript parser needs to be taught about "INSERT BEFORE/AFTER"...
        assert_eq!(
            &lds,
            indoc! {"
            
            /* Generated by postcard-infomem-host version 0.1.0 */

            SECTIONS {
                
                .info : {
                    _sinfo = .;
                    KEEP(*(.info))
                    _einfo = .;
                } > FLASH
            } INSERT AFTER .rodata
            
            
            "},
        );
    }

    #[test]
    fn generate_bare_section() {
        let cfg = BareSectionConfig::default()
            .set_max_size(Some(192))
            .set_memory_region("INFOMEM")
            .into();

        let lds = generate_script(cfg, "foo.x").unwrap();
        assert_ldscript_eq(
            &lds,
            indoc! {"
            SECTIONS {
                .info : {
                    _sinfo = .;
                    KEEP(*(.info))
                    _einfo = .;
                } > INFOMEM 
            }

            ASSERT((_einfo - _sinfo) <= 192, \"
            ERROR(foo.x): Information memory output section is greater than 192 bytes long.
            Flashing may overwrite important calibration data. The link has stopped as a precaution.
            \");
            "},
        )
        .unwrap();
    }
}
