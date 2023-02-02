#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::Debug;

use rustc_version::Channel;
use semver::Version;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

mod infostr;
pub use infostr::InfoStr;

mod magic;
pub use magic::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InfoMem<'a> {
    pub version: Version,
    #[serde(borrow)]
    pub user: UserInfo<'a>,
    pub rustc: RustcInfo<'a>,
}

impl<'a> Default for InfoMem<'a> {
    fn default() -> Self {
        InfoMem {
            // This will panic at compile time. If CARGO_PKG_VERSION fails to
            // parse at runtime (note that Version::parse is a const fn, and
            // the unwrap should be infallible), we have much bigger problems.
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            user: Default::default(),
            rustc: Default::default()
        }
    }
}

impl<'a> InfoMem<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct UserInfo<'a> {
    pub name: Option<InfoStr<'a>>,
    pub version: Option<Version>,
    #[serde(borrow)]
    pub git: Option<InfoStr<'a>>,
    pub build_date: Option<OffsetDateTime>,
}

impl<'a> Default for UserInfo<'a> {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
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

    impl From<Channel> for rustc_version::Channel {
        #[inline]
        fn from(other: Channel) -> rustc_version::Channel {
            match other {
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
pub struct RustcInfo<'a> {
    pub version: Option<Version>,
    pub llvm_version: Option<Version>,
    #[serde(with = "shim::channel_shim")]
    pub channel: Option<Channel>,
    #[serde(borrow)]
    pub git: Option<InfoStr<'a>>,
    pub host: Option<InfoStr<'a>>,
}

impl<'a> Default for RustcInfo<'a> {
    fn default() -> Self {
        Self {
            version: None,
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
