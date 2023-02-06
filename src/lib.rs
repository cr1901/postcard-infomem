/*! Core data types for serializing application and compiler-specific
information into a binary.
*/

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

/** Top-level container type for information intended to be embedded in a library
or binary.

This `struct` is likely to be filled in using e.g. [`generate_from_env`](../postcard_infomem_host/fn.generate_from_env.html)
from [`postcard_infomem_host`](../postcard_infomem_host/index.html),
or some other helper function. The information in this `struct` will change
for each crate that is compiled (_including the final binary application_).

The [`Default`] implementation provides a hardcoded [`InfoMem::version`], and [`Option::None`]
for all remaining `struct` members. _This crate does not attempt to populate
this `struct`._
*/
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InfoMem<'a, T = &'a [u8]> where T: sealed::Sealed {
    /** Version of this `struct` that was deserialized or created, hardcoded
    during crate compilation.
    
    The version is extracted from the `CARGO_PKG_VERSION` environment variable
    when [this crate](./index.html) is compiled. While the [`postcard`] format is [_not_ self-describing](https://postcard.jamesmunns.com/wire-format.html#non-self-describing-format),
    the wire format _is_ stable, and `struct` members are (de)serialized in order.
    _Therefore, this member must always remain first, even between major versions_.
    
    _It is inadvisable to manually alter this field._ The intent of this field
    is to allow backwards (and possibly forwards) compatibility with older
    (newer) versions of this `struct`. */
    pub version: Version,
    #[serde(borrow)]
    /// Information about the application where this `struct` originated.
    pub app: AppInfo<'a>,
    /// Information about the `rustc` compiler used to originally create this `struct`.
    pub rustc: RustcInfo<'a>,
    /** User-specific information to be included "as-is" (either `&[u8]`, `&mut [u8]`, or [`Vec<u8>`]).

    It is up to the user to ensure that the data contained in this field is
    parsed or deserialized by external means. */
    pub user: Option<T>
}

/** Private module intended to contrain the types of user-defined payloads
allowed in an [`InfoMem`]. */
pub(crate) mod sealed {
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    extern crate alloc;
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::vec::Vec;

    /// Trait to constrain the types of user-data that can be appended to an [`InfoMem`].
    pub trait Sealed {}

    impl Sealed for &[u8] {}
    impl Sealed for &mut [u8] {}
    #[cfg(feature = "alloc")]
    impl Sealed for Vec<u8> {}
}

impl<'a, T> Default for InfoMem<'a, T> where T: sealed::Sealed {
    fn default() -> Self {
        InfoMem {
            // This will panic at compile time. If CARGO_PKG_VERSION fails to
            // parse at runtime (note that Version::parse is a const fn, and
            // the unwrap should be infallible), we have much bigger problems.
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            app: Default::default(),
            rustc: Default::default(),
            user: Option::<T>::None
        }
    }
}

impl<'a, T> InfoMem<'a, T> where T: sealed::Sealed {
    /// Wrapper over the [`Default`] implementation.
    pub fn new() -> Self {
        Self::default()
    }
}

/** Information about the current crate being compiled.

This `struct` is likely to be filled in using e.g. [`generate_from_env`](../postcard_infomem_host/fn.generate_from_env.html)
from [`postcard_infomem_host`](../postcard_infomem_host/index.html),
or some other helper function. The information in this `struct` will change
for each crate that is compiled (_including the final binary application_).

The [`Default`] implementation provides [`Option::None`] for all `struct`
members. _This crate does not attempt to populate this `struct`._
*/
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AppInfo<'a> {
    /// Name of the current crate being compiled.
    pub name: Option<InfoStr<'a>>,
    /// [Semantic version](https://semver.org/) (semver) of the current crate being compiled.
    pub version: Option<Version>,
    #[serde(borrow)]
    /// Git commit of the source code of the current crate being compiled.
    pub git: Option<InfoStr<'a>>,
    /// Build date of the current crate being compiled.
    pub build_date: Option<OffsetDateTime>,
}

impl<'a> Default for AppInfo<'a> {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            git: Default::default(),
            build_date: Default::default(),
        }
    }
}

/// Helper types to ease the serialization process for types in external crates.
mod shim {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    /** [`enum`] representing the [release channel](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html),
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

    /** Module provided to [`serde`]s [`with`](https://serde.rs/remote-derive.html)
    attribute to implement the [`Serialize`] and [`Deserialize`] traits for
    the external [`rustc_version::Channel`]  type. */
    pub mod channel_shim {
        use super::*;

        /// Shim [`Deserialize`] implementation to deserialize [`rustc_version::Channel`] types.
        pub fn deserialize<'de, D>(d: D) -> Result<Option<rustc_version::Channel>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let foreign = <Option<Channel>>::deserialize(d)?;
            Ok(foreign.map(Into::into))
        }

        /// Shim [`Serialize`] implementation to serialize [`rustc_version::Channel`] types.
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

/** Information about the `rustc` compiler used to compile your application and
its dependencies.

This `struct` is likely to be filled in using e.g. [`generate_from_env`](../postcard_infomem_host/fn.generate_from_env.html)
from [`postcard_infomem_host`](../postcard_infomem_host/index.html),
or some other helper function. The [`Default`] implementation provides
[`Option::None`] for all `struct` members. _This crate does not attempt to
populate this `struct`._
*/
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RustcInfo<'a> {
    /// [Semantic version](https://semver.org/) (semver) of the `rustc` compiler.
    pub version: Option<Version>,
    /** [LLVM](https://llvm.org/) version used by the `rustc` compiler.
     
    _Although treated as a semver, older LLVM versions did not follow semver._ */
    pub llvm_version: Option<Version>,
    #[serde(with = "shim::channel_shim")]
    /// [Release channel](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html) of the `rustc` compiler.
    pub channel: Option<Channel>,
    #[serde(borrow)]
    /// Git commit of the source code used to build the `rustc` compiler.
    pub git: Option<InfoStr<'a>>,
    /// Host [triple](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) of the `rustc` compiler.
    pub host: Option<InfoStr<'a>>,
}

/// Create an empty [`RustcInfo`] with [`Option::None`]s, to be populated by external means.
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
        let im: InfoMem = InfoMem::default();

        let ser = to_allocvec(&im).unwrap();
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }

    #[test]
    fn round_trip_default_with_user_payload() {
        let mut im: InfoMem = InfoMem::default();
        let payload = [0, 1, 2, 3, 4];
        im.user = Some(&payload);

        let ser = to_allocvec(&im).unwrap();
        let de = from_bytes(&ser).unwrap();

        assert_eq!(im, de);
    }
}
