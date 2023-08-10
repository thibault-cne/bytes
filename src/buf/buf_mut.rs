use alloc::vec::Vec;
use core::{cmp, ptr};

use super::uninit_slice::UninitSlice;
use super::Buf;

pub trait BufMut {
    fn remaining_mut(&self) -> usize;

    fn has_remaining_mut(&self) -> bool {
        self.remaining_mut() > 0
    }

    fn chuncks_mut(&mut self) -> &mut UninitSlice;

    /// Advance the buffer of `count` bytes
    ///
    /// # Safety
    ///
    /// This functions is unsafe because it's not intended to be used directly. Instead use the
    /// `put` like functions.
    unsafe fn advance(&mut self, count: usize);

    fn put<T>(&mut self, mut src: T)
    where
        T: Buf,
        Self: Sized,
    {
        assert!(
            self.remaining_mut() >= src.remaining(),
            "not enough space remaining in BufMut: remaining: ({}) < needed ({})",
            self.remaining_mut(),
            src.remaining()
        );

        while src.has_remaining() {
            let chunck = src.chuncks();
            let dst = self.chuncks_mut();
            let count = cmp::min(chunck.len(), dst.len());

            unsafe { ptr::copy_nonoverlapping(chunck.as_ptr(), dst.as_mut_ptr(), count) };

            src.advance(count);
            unsafe {
                self.advance(count);
            }
        }
    }

    fn put_slice(&mut self, src: &[u8]) {
        let mut index = 0;

        assert!(
            self.remaining_mut() >= src.len(),
            "not enough space remaining in BufMut: remaining ({}) < needed ({})",
            self.remaining_mut(),
            src.len()
        );

        while index < src.len() {
            let count: usize;

            unsafe {
                let dst = self.chuncks_mut();
                count = cmp::min(dst.len(), src.len() - index);

                ptr::copy_nonoverlapping(src[index..].as_ptr(), dst.as_mut_ptr(), count);
                self.advance(count);
            }

            index += count;
        }
    }

    fn put_u8(&mut self, byte: u8) {
        let slice = [byte];
        self.put_slice(&slice);
    }
}

impl BufMut for Vec<u8> {
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
        let ptr = self.as_mut_ptr();

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
