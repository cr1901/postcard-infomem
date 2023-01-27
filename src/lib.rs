#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::string::String;

use rustc_version::Channel;
use semver;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InfoMem {
    pub version: Option<semver::Version>,
    pub user: UserInfo,
    pub rustc: RustcInfo,
}

impl InfoMem {
    pub fn new() -> Self {
        Self::default()
    }
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

// Helper types to ease the serialization process
mod shim {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    pub enum Channel {
        /// Development release channel
        Dev,
        /// Nightly release channel
        Nightly,
        /// Beta release channel
        Beta,
        /// Stable release channel
        Stable,
    }

    impl From<rustc_version::Channel> for Channel {
        #[inline]
        fn from(other: rustc_version::Channel) -> Self {
            match other {
                rustc_version::Channel::Dev => Channel::Dev,
                rustc_version::Channel::Nightly => Channel::Nightly,
                rustc_version::Channel::Beta => Channel::Beta,
                rustc_version::Channel::Stable => Channel::Stable,
            }
        }
    }

    impl Into<rustc_version::Channel> for Channel {
        #[inline]
        fn into(self) -> rustc_version::Channel {
            match self {
                Channel::Dev => rustc_version::Channel::Dev,
                Channel::Nightly => rustc_version::Channel::Nightly,
                Channel::Beta => rustc_version::Channel::Beta,
                Channel::Stable => rustc_version::Channel::Stable,
            }
        }
    }

    pub mod channel_shim {
        use super::*;

        pub fn deserialize<'de, D>(d: D) -> Result<Option<rustc_version::Channel>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let foreign = <Option<Channel>>::deserialize(d)?;
            Ok(foreign.map(Into::into))
        }

        pub fn serialize<S>(c: &Option<rustc_version::Channel>, s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let c = *c;
            c.map(<Channel as From<rustc_version::Channel>>::from)
                .serialize(s)
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RustcInfo {
    pub version: Option<semver::Version>,
    pub llvm_version: Option<semver::Version>,
    #[serde(with = "shim::channel_shim")]
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

#[cfg(test)]
mod tests {
    use crate::InfoMem;
    use postcard::{from_bytes, to_allocvec};

    extern crate std;

    #[test]
    fn round_trip_default() {
        let im = InfoMem::default();

        let ser = to_allocvec(&im).unwrap();
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }
}
