use crate::Bytes;

/// # Invariant
///
/// * `self.ptr` is always a valid pointer to a slice of bytes of len at least
/// `self.len`.
/// * `self.pos < self.len`
pub struct BytesIter {
    ptr: *const u8,
    len: usize,
    pos: usize,

    _b: Bytes,
}

impl BytesIter {
    #[inline]
    fn new(bytes: Bytes) -> BytesIter {
        // SAFETY + INVARIANT:
        // The `bytes` variable is stored in `self` to avoid the memory free.
        let ptr = unsafe { bytes.ptr() };
        let len = bytes.len();

        BytesIter {
            ptr,
            len,
            pos: 0,
            _b: bytes,
        }
    }

    /// Return the current position in the bytes buffer.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// The remaining len in the iterator.
    #[inline]
    pub fn len(&self) -> usize {
        self.len - self.pos
    }

    /// Check if the iterator is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos >= self.len
    }

    /// Peek the byte at the current position.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let b = Bytes::from_static(b"a byte slice");
    /// let iter = b.into_iter();
    ///
    /// assert_eq!(iter.peek(), Some(b'a'));
    /// ```
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        if !self.is_empty() {
            // SAFETY:
            // `self` is not empty
            unsafe { Some(*self.ptr.add(self.pos)) }
        } else {
            None
        }
    }

    /// Peek the byte at the nth position from the current position.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let b = Bytes::from_static(b"a byte slice");
    /// let iter = b.into_iter();
    ///
    /// assert_eq!(iter.peek_nth(0), Some(b'a'));
    /// assert_eq!(iter.peek_nth(3), Some(b'y'));
    /// ```
    #[inline]
    pub fn peek_nth(&self, n: usize) -> Option<u8> {
        let pos = self.pos + n;

        if pos < self.len {
            // SAFETY:
            // `pos < self.len`
            unsafe { Some(*self.ptr.add(pos)) }
        } else {
            None
        }
    }

    /// Peek a slice of bytes from `self.pos` to `self.pos + n`.
    /// If `self.pos + n >= self.len` then `Option::None` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    /// let b = Bytes::from_static(b"a bytes slice");
    /// let iter = b.into_iter();
    ///    
    /// assert_eq!(iter.peek_n(7), Some(&b"a bytes"[..]));
    /// ```
    #[inline]
    pub fn peek_n(&self, n: usize) -> Option<&[u8]> {
        let end = self.pos + n;

        if end < self.len {
            Some(&self._b[self.pos..end])
        } else {
            None
        }
    }

    /// Take the next bytes from `self.pos` to `self.pos + n`.
    /// If `self.pos + n >= self.len` then `Option::None` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let b = Bytes::from_static(b"a bytes slice");
    /// let mut iter = b.into_iter();
    ///
    /// assert_eq!(iter.next_n(7), Some(&b"a bytes"[..]));
    /// assert_eq!(iter.next(), Some(b' '));
    /// ```
    #[inline]
    pub fn next_n(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.pos + n;

        if end < self.len {
            let b = Some(&self._b[self.pos..end]);
            self.pos += n;
            b
        } else {
            None
        }
    }

    /// Advance the position cursor of `n`.
    ///
    /// # Safety
    ///
    /// You must ensures that `self.pos + n < self.len` in order to keep `Self`
    /// invariant.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let b = Bytes::from_static(b"a bytes slice");
    /// let mut iter = b.into_iter();
    ///
    /// unsafe { iter.advance(6) };
    /// assert_eq!(iter.next(), Some(b's'));
    /// ```
    #[inline]
    pub unsafe fn advance(&mut self, n: usize) {
        debug_assert!(
            self.pos + n < self.len,
            "position out of bounds, self.pos ({}) >= self.len ({})",
            self.pos + n,
            self.len
        );
        self.pos += n;
    }

    /// Advance the position cursor of 1. This is strictly equivalent to
    /// `advance(1)`.
    ///
    /// # Safety
    ///
    /// You must ensures that `self.pos + 1 < self.len` in order to keep `Self`
    /// invariant.
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let b = Bytes::from_static(b"a bytes slice");
    /// let mut iter = b.into_iter();
    ///
    /// unsafe { iter.bump() };
    /// assert_eq!(iter.next(), Some(b' '));
    /// ```
    #[inline]
    pub unsafe fn bump(&mut self) {
        self.advance(1)
    }
}

impl IntoIterator for Bytes {
    type Item = u8;
    type IntoIter = BytesIter;

    #[inline]
    fn into_iter(self) -> BytesIter {
        BytesIter::new(self)
    }
}

impl Iterator for BytesIter {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.pos < self.len {
            // SAFETY:
            // `self.ptr` is valid by the `self` invariant and `self.pos < self.len`
            let b = unsafe { Some(*self.ptr.add(self.pos)) };
            self.pos += 1;
            b
        } else {
            None
        }
    }
}
