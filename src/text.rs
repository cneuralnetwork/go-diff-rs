use std::borrow::Cow;
use std::fmt;
use std::ops::{Deref, DerefMut};

/// A Go-compatible byte string.
///
/// Go strings may contain arbitrary bytes, while Rust `str` values must be valid UTF-8.
/// `Text` keeps the exact bytes and offers UTF-8 conveniences when they are valid.
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Text(Vec<u8>);

impl Text {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn as_mut_bytes(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.0)
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl AsRef<[u8]> for Text {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Text {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for Text {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for Text {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Text {
    fn from(value: &[u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &byte in &self.0 {
            for escaped in std::ascii::escape_default(byte) {
                write!(f, "{}", char::from(escaped))?;
            }
        }
        write!(f, "\"")
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}
