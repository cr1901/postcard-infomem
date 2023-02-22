//! Helper types to ease the serialization process for types in external crates.

include!(concat!(env!("OUT_DIR"), "/version.rs"));

use core::fmt;
#[cfg(feature = "std")]
use std::error::Error as StdError;

#[cfg(feature = "alloc")]
use semver;
use serde::{Deserialize, Serialize};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::string::ToString;

#[cfg(feature = "std")]
use rustc_version;

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
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
    #[serde(borrow)]
    pub pre: Option<InfoStr<'a>>,
    #[serde(borrow)]
    pub build: Option<InfoStr<'a>>,
}

impl<'a> Semver<'a> {
    pub(super) const fn this_version() -> Self {
        Self {
            major: PIM_VERSION_MAJOR,
            minor: PIM_VERSION_MINOR,
            patch: PIM_VERSION_PATCH,
            pre: PIM_VERSION_PRE,
            build: None,
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
        let major = usize::try_from(value.major)
            .map_err(|_| TryFromVersionError("semver major version exceeds usize"))?;
        let minor = usize::try_from(value.minor)
            .map_err(|_| TryFromVersionError("semver minor version exceeds usize"))?;
        let patch = usize::try_from(value.patch)
            .map_err(|_| TryFromVersionError("semver patch version exceeds usize"))?;

        let pre = match value.pre.as_str() {
            "" => None,
            s => Some(InfoStr::Owned(s.to_string())),
        };

        let build = match value.build.as_str() {
            "" => None,
            s => Some(InfoStr::Owned(s.to_string())),
        };

        Ok(Self {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }
}
