use core::ptr::{self, NonNull};

use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};

pub struct BytesMut {
    ptr: NonNull<u8>,
    len: usize,
    cap: usize,
}

impl BytesMut {
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
            cap as isize <= isize::MAX,
            "capacity too large, capacity must be inferior to `isize::MAX`"
        );

        let layout = Layout::array::<u8>(cap).unwrap();
        let ptr = unsafe { alloc(layout) };

        let ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };

        BytesMut {
            ptr,
            cap: cap as usize,
            len: 0,
        }
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
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { realloc(old_ptr, old_layout, layout.size()) }
        };

        self.ptr = match NonNull::new(ptr as *mut u8) {
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

unsafe impl Sync for BytesMut {}

unsafe impl Send for BytesMut {}

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
