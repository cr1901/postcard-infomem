//! Helper types to ease the serialization process for types in external crates.

use serde::{Deserialize, Serialize};

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
