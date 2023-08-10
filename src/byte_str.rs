use core::str;

use crate::Bytes;

/// This reprensent a `Bytes` but with only valid utf8.
///
/// # Invariant
///
/// * The inner `Bytes` buffer is always made of valid utf8 bytes
pub struct ByteStr {
    inner: Bytes,
}

impl ByteStr {
    /// Create a new `ByteStr`
    ///
    /// # Invariant
    ///
    /// The inner buffer is empty so it's made of valid utf8
    pub fn new() -> ByteStr {
        ByteStr {
            inner: Bytes::new(),
        }
    }

    /// Create a new `ByteStr` from a `&'static str`
    ///
    /// # Invariant
    ///
    /// Rust ensures that strings are made of valid utf8 so `src.as_bytes()` is made of valid utf8
    #[inline]
    pub const fn from_static(src: &'static str) -> ByteStr {
        ByteStr {
            inner: Bytes::from_static(src.as_bytes()),
        }
    }

    /// Create a new `ByteStr` from an unchecked bytes slice
    ///
    /// # Safety
    ///
    /// This functions is unsafe because it requires valid utf8 bytes to ensure the `ByteStr`
    /// invariant.
    ///
    /// # Panics
    ///
    /// In debug mode this function will panic if the given bytes are invalid utf8. In release mode
    /// this will result in undefined behaviour.
    pub unsafe fn from_utf8_unchecked(src: &[u8]) -> ByteStr {
        if cfg!(debug_assert) {
            match str::from_utf8(src) {
                Ok(_) => ByteStr {
                    inner: Bytes::copy_from_slice(src),
                },
                Err(e) => panic!("invalid uft8: {}", e),
            }
        } else {
            ByteStr {
                inner: Bytes::copy_from_slice(src),
            }
        }
    }

    /// Create a new `ByteStr` from an unchecked `Bytes`
    ///
    /// # Safety
    ///
    /// This functions is unsafe because it requires valid utf8 bytes to ensure the `ByteStr`
    /// invariant.
    ///
    /// # Panics
    ///
    /// In debug mode this function will panic if the given bytes are invalid utf8. In release mode
    /// this will result in undefined behaviour.
    pub unsafe fn from_shared_unsafe(src: Bytes) -> ByteStr {
        if cfg!(debug_assert) {
            match str::from_utf8(&src) {
                Ok(_) => ByteStr { inner: src },
                Err(e) => panic!("invalid utf8: {}", e),
            }
        } else {
            ByteStr { inner: src }
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // Safety: the invariant of `ByteStr` ensures that inner is made of valid utf8
        unsafe { str::from_utf8_unchecked(&self.inner) }
    }
}

impl Default for ByteStr {
    fn default() -> ByteStr {
        ByteStr::new()
    }
}

impl From<String> for ByteStr {
    fn from(value: String) -> ByteStr {
        // Safety: Rust ensures that all string are made of valid utf8 bytes
        unsafe { ByteStr::from_utf8_unchecked(value.as_bytes()) }
    }
}

impl<'a> From<&'a str> for ByteStr {
    fn from(value: &'a str) -> ByteStr {
        // Safety: Rust ensures that all string are made of valid utf8 bytes
        unsafe { ByteStr::from_utf8_unchecked(value.as_bytes()) }
    }
}

impl AsRef<str> for ByteStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_static() {
        let bytes = ByteStr::from_static("this is valid utf8");

        assert_eq!("this is valid utf8", bytes.as_str());
    }

    #[test]
    fn from_string() {
        let bytes = ByteStr::from(String::from("this is a string"));

        assert_eq!("this is a string", bytes.as_str());
    }
}
