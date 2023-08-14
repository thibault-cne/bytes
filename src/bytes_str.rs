use core::{fmt, ops, str};

use crate::Bytes;

/// This reprensent a `Bytes` but with only valid utf8.
///
/// # Invariant
///
/// * The inner `Bytes` buffer is always made of valid utf8 bytes
#[derive(Clone, Eq, PartialEq)]
pub struct BytesStr {
    inner: Bytes,
}

impl BytesStr {
    /// Create a new `BytesStr`
    ///
    /// # Invariant
    ///
    /// The inner buffer is empty so it's made of valid utf8
    pub fn new() -> BytesStr {
        BytesStr {
            inner: Bytes::new(),
        }
    }

    /// Create a new `BytesStr` from a `&'static str`
    ///
    /// # Invariant
    ///
    /// Rust ensures that strings are made of valid utf8 so `src.as_bytes()` is made of valid utf8
    #[inline]
    pub const fn from_static(src: &'static str) -> BytesStr {
        BytesStr {
            inner: Bytes::from_static(src.as_bytes()),
        }
    }

    /// Create a new `BytesStr` from an unchecked bytes slice
    ///
    /// # Safety
    ///
    /// This functions is unsafe because it requires valid utf8 bytes to ensure the `BytesStr`
    /// invariant.
    ///
    /// # Panics
    ///
    /// In debug mode this function will panic if the given bytes are invalid utf8. In release mode
    /// this will result in undefined behaviour.
    pub unsafe fn from_utf8_unchecked(src: &[u8]) -> BytesStr {
        if cfg!(debug_assert) {
            match str::from_utf8(src) {
                Ok(_) => BytesStr {
                    inner: Bytes::copy_from_slice(src),
                },
                Err(e) => panic!("invalid uft8: {}", e),
            }
        } else {
            BytesStr {
                inner: Bytes::copy_from_slice(src),
            }
        }
    }

    /// Create a new `BytesStr` from an unchecked `Bytes`
    ///
    /// # Safety
    ///
    /// This functions is unsafe because it requires valid utf8 bytes to ensure the `BytesStr`
    /// invariant.
    ///
    /// # Panics
    ///
    /// In debug mode this function will panic if the given bytes are invalid utf8. In release mode
    /// this will result in undefined behaviour.
    pub unsafe fn from_shared_unchecked(src: Bytes) -> BytesStr {
        if cfg!(debug_assert) {
            match str::from_utf8(&src) {
                Ok(_) => BytesStr { inner: src },
                Err(e) => panic!("invalid utf8: {}", e),
            }
        } else {
            BytesStr { inner: src }
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // Safety: the invariant of `BytesStr` ensures that inner is made of valid utf8
        unsafe { str::from_utf8_unchecked(&self.inner) }
    }
}

impl Default for BytesStr {
    fn default() -> BytesStr {
        BytesStr::new()
    }
}

impl From<String> for BytesStr {
    fn from(value: String) -> BytesStr {
        // Safety: Rust ensures that all string are made of valid utf8 bytes
        unsafe { BytesStr::from_utf8_unchecked(value.as_bytes()) }
    }
}

impl<'a> From<&'a str> for BytesStr {
    fn from(value: &'a str) -> BytesStr {
        // Safety: Rust ensures that all string are made of valid utf8 bytes
        unsafe { BytesStr::from_utf8_unchecked(value.as_bytes()) }
    }
}

impl AsRef<str> for BytesStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for BytesStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BytesStr")
            .field("inner", &self.as_str())
            .finish()
    }
}

impl fmt::Display for BytesStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<BytesStr> for Bytes {
    fn from(value: BytesStr) -> Bytes {
        value.inner
    }
}

impl ops::Deref for BytesStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_static() {
        let bytes = BytesStr::from_static("this is valid utf8");

        assert_eq!("this is valid utf8", bytes.as_str());
    }

    #[test]
    fn from_string() {
        let bytes = BytesStr::from(String::from("this is a string"));

        assert_eq!("this is a string", bytes.as_str());
    }

    #[test]
    fn format() {
        let bytes = BytesStr::from_static("this is a BytesStr");

        assert_eq!("this is a BytesStr", format!("{}", bytes));
    }
}
