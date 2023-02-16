//! Helper types to ease the serialization process for types in external crates.

use core::fmt;
#[cfg(feature = "std")]
use std::error::Error as StdError;

use embedded_semver::Semver as CoreSemver;
use konst::{primitive::parse_usize, result::unwrap_ctx};

#[cfg(feature = "alloc")]
use semver;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "std")]
use rustc_version;

use crate::InfoStr;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
/** [`enum`] representing the [release channel](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html)
of the `rustc` compiler.

This enum is created from a [`rustc_version::Channel`] using its [`From`]
implementation, and exists mainly to aid in [deriving](https://serde.rs/remote-derive.html)
the [`Serialize`] and [`Deserialize`] traits for [`RustcInfo`].
*/
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

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Semver<'a> {
    #[serde(with = "coresemver")]
    pub core: CoreSemver,
    #[serde(borrow)]
    pub pre: Option<InfoStr<'a>>,
    #[serde(borrow)]
    pub build: Option<InfoStr<'a>>
}

impl<'a> Semver<'a> {
    pub(super) fn this_version() -> Self {
        let core = CoreSemver::new(unwrap_ctx!(parse_usize(env!("CARGO_PKG_VERSION_MAJOR"))),
            unwrap_ctx!(parse_usize(env!("CARGO_PKG_VERSION_MINOR"))),
            unwrap_ctx!(parse_usize(env!("CARGO_PKG_VERSION_PATCH"))));
        

        let pre = match env!("CARGO_PKG_VERSION_PRE") {
            "" => None,
            s => Some(InfoStr::Borrowed(s))
        };

        Self {
            core,
            pre,
            build: None
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct TryFromVersionError(&'static str);

impl fmt::Display for TryFromVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "std")]
impl StdError for TryFromVersionError {}

#[cfg(feature = "alloc")]
impl<'a> TryFrom<semver::Version> for Semver<'a> {
    type Error = TryFromVersionError;

    fn try_from(value: semver::Version) -> Result<Self, Self::Error> {
        let core = embedded_semver::Semver::new(
            usize::try_from(value.major).map_err(|_| TryFromVersionError("semver major version exceeds usize"))?,
            usize::try_from(value.minor).map_err(|_| TryFromVersionError("semver minor version exceeds usize"))?,
            usize::try_from(value.patch).map_err(|_| TryFromVersionError("semver patch version exceeds usize"))?,
        );

        // We only want to parse as u32 to confirm this'll serialize properly.
        let _ = core.to_u32().map_err(|_| TryFromVersionError("semver cannot be encoded into u32"))?;

        let pre = match value.pre.as_str() {
            "" => None,
            s => Some(InfoStr::Owned(s.to_string()))
        };

        let build = match value.pre.as_str() {
            "" => None,
            s => Some(InfoStr::Owned(s.to_string()))
        };

        Ok(Self {
            core,
            pre,
            build
        })
    }
}

mod coresemver {
    use core::fmt;

    use super::*;

    use embedded_semver::Error;
    use serde::de::{self, Visitor};
    use serde::ser;

    struct CoreVisitor;

    // This Visitor is only compatible with Postcard at present (e.g. )
    impl<'de> Visitor<'de> for CoreVisitor {
        type Value = u32;
    
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer between -2^31 and 2^31")
        }
    
        fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(u32::from(value))
        }

        fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(u32::from(value))
        }
    
        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value >= u64::from(u32::MIN) && value <= u64::from(u32::MAX) {
                Ok(value as u32)
            } else {
                Err(E::custom("core version info is invalid- it did not fit into 32-bits"))
            }
        }
    }

    /// Shim [`Deserialize`] implementation to deserialize [`embedded_semver::Semver`] types.
    pub fn deserialize<'de, D>(d: D) -> Result<embedded_semver::Semver, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = d.deserialize_u32(CoreVisitor)?;
        Ok(CoreSemver::from_u32(wire).map_err(|e|
            match e {
                Error::Overflow => de::Error::custom("semver major, minor, or patch field overflow"),
                Error::UnknownMagic(_) => de::Error::custom("unknown magic constant"),
                Error::UnsupportedMagic(_) => de::Error::custom("unsupported magic constant")
            }
        )?)
    }

    /// Shim [`Serialize`] implementation to serialize [`embedded_semver::Semver`] types.
    pub fn serialize<S>(c: &embedded_semver::Semver, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        c.to_u32().map_err(|e|
            match e {
                Error::Overflow => ser::Error::custom("semver major, minor, or patch field overflow"),
                Error::UnknownMagic(_) => ser::Error::custom("unknown magic constant"),
                Error::UnsupportedMagic(_) => ser::Error::custom("unsupported magic constant")
            }
        )?.serialize(s)
    }
}
