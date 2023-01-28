//! A fancy container for owned or borrowed strings
//!
//! So, sometimes you want to use owned types, and have the std
//! library. And other times you don't, and borrowed types are
//! okay. This handles both cases, based on a feature flag.
//!
//! Inspired by @whitequark's `managed` crate.

use core::fmt::Debug;
use serde::{Deserialize, Serialize};

pub enum InfoStr<'a> {
    Borrowed(&'a str),
    #[cfg(feature = "std")]
    Owned(String),
}

impl<'a> InfoStr<'a> {
    /// Create an InfoStr from a borrowed str
    pub fn from_borrowed(stir: &'a str) -> Self {
        InfoStr::Borrowed(stir)
    }

    /// Create an InfoStr from an owned String
    #[cfg(feature = "std")]
    pub fn from_string(stir: String) -> InfoStr<'static> {
        InfoStr::Owned(stir)
    }

    /// View the InfoStr as a str
    pub fn as_str(&'a self) -> &'a str {
        match self {
            InfoStr::Borrowed(s) => s,
            #[cfg(feature = "std")]
            InfoStr::Owned(s) => s.as_str(),
        }
    }
}

// Implement a couple traits by passing through to &str's methods

impl<'a> PartialEq for InfoStr<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<'a> Debug for InfoStr<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<'a> Serialize for InfoStr<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for InfoStr<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let stir = <&'de str as Deserialize<'de>>::deserialize(deserializer)?;
        Ok(InfoStr::Borrowed(stir))
    }
}
