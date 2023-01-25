#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use rustc_version::Channel;
use semver;
use serde::{
    de::{self, SeqAccess, Visitor},
    ser::{self, SerializeStruct},
    Deserialize, Serialize,
};
use time::{format_description, OffsetDateTime};

#[derive(Debug, PartialEq)]
pub struct InfoMem {
    pub version: Option<semver::Version>,
    pub user: UserInfo,
    pub rustc: RustcInfo,
}

impl Default for InfoMem {
    fn default() -> Self {
        Self {
            version: None,
            user: Default::default(),
            rustc: Default::default(),
        }
    }
}

struct InfoMemVisitor;

impl<'de> Visitor<'de> for InfoMemVisitor {
    type Value = InfoMem;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("struct InfoMem")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<InfoMem, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let version_str: Option<&str> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let version: Option<semver::Version> = if let Some(s) = version_str {
            Some(
                semver::Version::parse(s)
                    .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))?,
            )
        } else {
            None
        };

        let user = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let rustc = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;

        Ok(InfoMem {
            version,
            user,
            rustc,
        })
    }
}

impl<'de> Deserialize<'de> for InfoMem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct("InfoMem", &["version", "user", "rustc"], InfoMemVisitor)
    }
}

impl Serialize for InfoMem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("InfoMem", 3)?;
        state.serialize_field("version", &self.version.as_ref().map(|v| v.to_string()))?;
        state.serialize_field("user", &self.user)?;
        state.serialize_field("rustc", &self.rustc)?;

        state.end()
    }
}

#[derive(Debug, PartialEq)]
pub struct UserInfo {
    pub version: Option<semver::Version>,
    pub git: Option<String>,
    pub build_date: Option<OffsetDateTime>,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            version: semver::Version::parse(env!("CARGO_PKG_VERSION"))
                .map(|v| Some(v))
                .unwrap_or(None),
            git: Default::default(),
            build_date: Default::default(),
        }
    }
}

struct UserInfoVisitor;

impl<'de> Visitor<'de> for UserInfoVisitor {
    type Value = UserInfo;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("struct UserInfo")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<UserInfo, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let version_str: Option<&str> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let version: Option<semver::Version> = if let Some(s) = version_str {
            Some(
                semver::Version::parse(s)
                    .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))?,
            )
        } else {
            None
        };

        let git = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let build_date_str = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;
        let build_date: Option<OffsetDateTime> = if let Some(s) = build_date_str {
            let format = format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
                     sign:mandatory]:[offset_minute]:[offset_second]",
            )
            .unwrap();

            Some(
                OffsetDateTime::parse(s, &format)
                    .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))?,
            )
        } else {
            None
        };

        Ok(UserInfo {
            version,
            git,
            build_date,
        })
    }
}

impl<'de> Deserialize<'de> for UserInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "UserInfo",
            &["version", "git", "build_date"],
            UserInfoVisitor,
        )
    }
}

impl Serialize for UserInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("InfoMem", 3)?;
        state.serialize_field("version", &self.version.as_ref().map(|v| v.to_string()))?;
        state.serialize_field("git", &self.git)?;

        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
                 sign:mandatory]:[offset_minute]:[offset_second]",
        )
        .unwrap();

        let build_date = match &self.build_date {
            Some(d) => Some(d.format(&format).map_err(|e| ser::Error::custom(e))?),
            None => None,
        };

        state.serialize_field("build_date", &build_date)?;

        state.end()
    }
}

#[derive(Debug, PartialEq)]
pub struct RustcInfo {
    pub version: Option<semver::Version>,
    pub llvm_version: Option<semver::Version>,
    pub channel: Option<Channel>,
    pub git: Option<String>,
    pub host: Option<String>,
}

impl Default for RustcInfo {
    fn default() -> Self {
        Self {
            version: semver::Version::parse(env!("CARGO_PKG_VERSION"))
                .map(|v| Some(v))
                .unwrap_or(None),
            llvm_version: None,
            channel: None,
            git: Default::default(),
            host: Default::default(),
        }
    }
}

struct RustcInfoVisitor;

impl<'de> Visitor<'de> for RustcInfoVisitor {
    type Value = RustcInfo;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("struct RustcInfo")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<RustcInfo, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let version_str: Option<&str> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let version: Option<semver::Version> = if let Some(s) = version_str {
            Some(
                semver::Version::parse(s)
                    .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))?,
            )
        } else {
            None
        };

        let version_str: Option<&str> = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let llvm_version: Option<semver::Version> = if let Some(s) = version_str {
            Some(
                semver::Version::parse(s)
                    .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(s), &self))?,
            )
        } else {
            None
        };

        let channel = match seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?
        {
            "dev" => Some(Channel::Dev),
            "nightly" => Some(Channel::Nightly),
            "beta" => Some(Channel::Beta),
            "stable" => Some(Channel::Stable),
            "unknown" => None,
            s => {
                return Err(de::Error::invalid_value(
                    serde::de::Unexpected::Str(s),
                    &self,
                ))
            }
        };

        let git = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(3, &self))?;
        let host = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(4, &self))?;

        Ok(RustcInfo {
            version,
            llvm_version,
            channel,
            git,
            host,
        })
    }
}

impl<'de> Deserialize<'de> for RustcInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "RustcInfo",
            &["version", "llvm_version", "channel", "git", "host"],
            RustcInfoVisitor,
        )
    }
}

impl Serialize for RustcInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("InfoMem", 3)?;
        state.serialize_field("version", &self.version.as_ref().map(|v| v.to_string()))?;
        state.serialize_field(
            "llvm_version",
            &self.llvm_version.as_ref().map(|v| v.to_string()),
        )?;

        let ch_str = match self.channel {
            Some(Channel::Dev) => "dev",
            Some(Channel::Nightly) => "nightly",
            Some(Channel::Beta) => "beta",
            Some(Channel::Stable) => "stable",
            None => "unknown",
        };
        state.serialize_field("channel", ch_str)?;

        state.serialize_field("git", &self.git)?;
        state.serialize_field("host", &self.host)?;

        state.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::InfoMem;
    use git_version::git_version;
    use postcard::{from_bytes, to_allocvec};
    use rustc_version::version_meta;
    use time::OffsetDateTime;

    extern crate std;
    use std::string::ToString;
    use std::print;

    #[test]
    fn round_trip_default() {
        let im = InfoMem::default();

        let ser = to_allocvec(&im).unwrap();
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn round_trip_filled() {
        let mut im = InfoMem::default();

        // CARGO_PKG_VERSION hardcoded while compiling this crate.
        im.version = Some(semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap());

        // CARGO_PKG_VERSION comes from the parent.
        im.user.version =
            Some(semver::Version::parse(&std::env::var("CARGO_PKG_VERSION").unwrap()).unwrap());
        im.user.git = Some(git_version!(fallback = "unknown").to_string());
        im.user.build_date = Some(OffsetDateTime::now_local().unwrap().replace_millisecond(0).unwrap());

        if let Ok(rv) = version_meta() {
            im.rustc.version = Some(rv.semver);
            im.rustc.llvm_version = rv
                .llvm_version
                .map(|l| semver::Version::new(l.major, l.minor, 0));
            im.rustc.git = rv.commit_hash;
            im.rustc.host = Some(rv.host);
            im.rustc.channel = Some(rv.channel);
        }

        let ser = to_allocvec(&im).unwrap();
        ser.iter().for_each(|b| {
            print!("{:02X} ", b);
        });
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }
}
