use core::fmt;
use core::ptr::{self, NonNull};
use core::slice;

use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};

use crate::buf::{Buf, BufMut, UninitSlice};

pub struct BytesMut {
    ptr: NonNull<u8>,
    len: usize,
    cap: usize,
}

impl BytesMut {
    #[inline]
    pub fn new() -> BytesMut {
        BytesMut {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    /// Create an empty `bytes::BytesMut` with a given capacity. Given `cap` must be inferior
    /// to `isize::MAX`.
    ///
    /// # Panics
    ///
    /// If the `cap` exceed `isize::MAX` the function will panic.
    ///
    /// ```should_panic
    /// use bytes::BytesMut;
    ///
    /// // usize::MAX > isize::MAX so this should panic
    /// let _ = BytesMut::with_capacity(usize::MAX);
    /// ```
    pub fn with_capacity(cap: usize) -> BytesMut {
        assert!(
            cap <= isize::MAX as usize,
            "capacity too large, capacity must be inferior to `isize::MAX`"
        );

        let layout = Layout::array::<u8>(cap).unwrap();
        let ptr = unsafe { alloc(layout) };

        let ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };

        BytesMut { ptr, cap, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn push(&mut self, b: u8) {
        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len), b);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }

    /// Consume `self` and turns it into a `Vec<u8>`
    pub fn to_vec(self) -> alloc::vec::Vec<u8> {
        // Create the vec from ptr
        let v = unsafe { alloc::vec::Vec::from_raw_parts(self.ptr.as_ptr(), self.len, self.cap) };

        // Forget `self` to avoid running it's destructor
        core::mem::forget(self);
        v
    }

    #[inline]
    pub fn freeze(self) -> crate::bytes::Bytes {
        self.to_vec().into()
    }

    /// Set the len of `self` to `len`
    ///
    /// # Safety
    ///
    /// * `len` must be inferior or equal to `self.cap`
    /// * all bytes in range `0..len` must be initialized or this will lead to **undefined
    /// behaviours**.
    ///
    /// # Panics
    ///
    /// This will panic in `debug` builds if `len > self.cap`. In production code it won't panic
    /// but lead to **undefined behaviours**.
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.cap);

        self.len = len;
    }

    /// Extends `self` with the given `slice`
    ///
    /// # Example
    ///
    /// ```
    /// use bytes::BytesMut;
    ///
    /// let mut bytes_mut = BytesMut::with_capacity(10);
    ///
    /// bytes_mut.extend_from_slice(b"10 bytes !");
    ///
    /// assert_eq!(bytes_mut.len(), 10);
    /// assert_eq!(bytes_mut.capacity(), 10);
    /// assert_eq!(bytes_mut.as_ref(), b"10 bytes !");
    /// ```
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.reserve(slice.len());

        // SAFETY:
        // The reserve calls ensures that we have enough space in `self.ptr` to copy every bytes of
        // `slice.as_ptr()`

        unsafe {
            self.ptr
                .as_ptr()
                .add(self.len)
                .copy_from(slice.as_ptr(), slice.len());
        }
        self.len += slice.len();
    }

    #[inline]
    pub fn reserve(&mut self, res: usize) {
        let rem = self.cap - self.len;

        if rem >= res {
            return;
        }

        self.inner_reserve(self.cap + (res - rem));
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        if self.cap == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
    }

    fn inner_reserve(&mut self, cap: usize) {
        assert!(cap <= isize::MAX as usize, "capacity too large");

        let layout = Layout::array::<u8>(cap).unwrap();

        let ptr = if self.cap == 0 {
            unsafe { alloc(layout) }
        } else {
            let old_layout = Layout::array::<u8>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr();

            unsafe { realloc(old_ptr, old_layout, layout.size()) }
        };

        self.ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };
        self.cap = cap;
    }

    fn grow(&mut self) {
        let (cap, layout) = if self.cap == 0 {
            (1, Layout::array::<u8>(1).unwrap())
        } else {
            let new_cap = 2 * self.cap;

            (new_cap, Layout::array::<u8>(new_cap).unwrap())
        };

        assert!(cap <= isize::MAX as usize, "allocation too large");

        let ptr = if self.cap == 0 {
            unsafe { alloc(layout) }
        } else {
            let old_layout = Layout::array::<u8>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr();
            unsafe { realloc(old_ptr, old_layout, layout.size()) }
        };

        self.ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };
        self.cap = cap;
    }
}

impl Drop for BytesMut {
    fn drop(&mut self) {
        if self.cap != 0 {
            let layout = Layout::array::<u8>(self.cap).unwrap();
            unsafe { dealloc(self.ptr.as_ptr(), layout) };
        }
    }
}

impl Default for BytesMut {
    #[inline]
    fn default() -> BytesMut {
        BytesMut::new()
    }
}

unsafe impl Sync for BytesMut {}

unsafe impl Send for BytesMut {}

// === impl `bytes::BufMut` ===

impl BufMut for BytesMut {
    fn remaining_mut(&self) -> usize {
        // `alloc::vec` ensures that vectors don't allocate more than `isize::MAX` bytes
        core::isize::MAX as usize - self.len()
    }

    unsafe fn advance(&mut self, count: usize) {
        let len = self.len();
        let rem = self.capacity() - len;

        assert!(
            count <= rem,
            "not enough space to advance: remaining ({}) < count ({})",
            rem,
            count
        );

        self.set_len(len + count);
    }

    fn chuncks_mut(&mut self) -> &mut UninitSlice {
        let cap = self.capacity();
        let len = self.len();
        let ptr = self.ptr.as_ptr();

        unsafe { &mut UninitSlice::from_raw_parts(ptr, cap)[len..] }
    }

    fn put<T>(&mut self, mut src: T)
    where
        T: Buf,
        Self: Sized,
    {
        self.reserve(src.remaining());

        while src.has_remaining() {
            let chunck = src.chuncks();

            self.extend_from_slice(chunck);
            src.advance(chunck.len());
        }
    }

    #[inline]
    fn put_slice(&mut self, src: &[u8]) {
        self.extend_from_slice(src);
    }
}

// === AsRef / Deref / Borrow ===

impl AsRef<[u8]> for BytesMut {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

// === Write ===

impl fmt::Write for BytesMut {
    #[inline]
    fn write_str(&mut self, src: &str) -> fmt::Result {
        if self.remaining_mut() >= self.len() {
            self.put_slice(src.as_bytes());
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        fmt::write(self, args)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new() {
        let bytes_mut = BytesMut::new();

        assert_eq!(bytes_mut.len, 0);
        assert_eq!(bytes_mut.cap, 0);
    }

    #[test]
    fn with_capacity() {
        let bytes_mut = BytesMut::with_capacity(10);

        assert_eq!(bytes_mut.len, 0);
        assert_eq!(bytes_mut.cap, 10);
    }

    #[test]
    fn to_vec() {
        let mut bytes_mut = BytesMut::with_capacity(10);

        bytes_mut.push(0);
        bytes_mut.push(0);
        bytes_mut.push(0);
        bytes_mut.push(0);

        let vec = bytes_mut.to_vec();

        assert_eq!(vec.capacity(), 10);
        assert_eq!(vec.len(), 4);
        assert!(vec.contains(&0));
    }
}
