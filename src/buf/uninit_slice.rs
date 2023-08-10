use core::mem::MaybeUninit;
use core::ops::{Index, IndexMut, Range, RangeFrom, RangeFull};

pub struct UninitSlice([MaybeUninit<u8>]);

impl UninitSlice {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn from_slice(slice: &mut [MaybeUninit<u8>]) -> &mut UninitSlice {
        unsafe { &mut *(slice as *mut [MaybeUninit<u8>] as *mut UninitSlice) }
    }

    pub unsafe fn from_raw_parts<'a>(ptr: *mut u8, len: usize) -> &'a mut UninitSlice {
        let slice: &mut [MaybeUninit<u8>] = core::slice::from_raw_parts_mut(ptr as _, len);
        UninitSlice::from_slice(slice)
    }

    /// Copy a slice from `src` into `self`
    ///
    /// # Example
    ///
    /// ```
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `self` and `src` has different len
    pub fn copy_from_slice(&mut self, src: &[u8]) {
        use core::ptr;

        assert!(
            self.len() == src.len(),
            "self and src have different len: self ({}) != src ({}))",
            self.len(),
            src.len()
        );

        unsafe { ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr(), self.len()) }
    }

    pub fn write_byte(&mut self, index: usize, byte: u8) {
        assert!(
            index < self.len(),
            "index out of bounds: index ({}) >= len ({})",
            index,
            self.len()
        );

        unsafe { self[index..].as_mut_ptr().write(byte) }
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr() as *mut u8
    }
}

macro_rules! impl_index {
    ($($ty:ty),*) => {
       $(
            impl Index<$ty> for UninitSlice {
                type Output = UninitSlice;

                fn index(&self, index: $ty) -> &UninitSlice {
                    let indexed = &self.0[index];
                    unsafe { &*(indexed as *const [MaybeUninit<u8>] as *const UninitSlice) }
                }
            }

            impl IndexMut<$ty> for UninitSlice {
                fn index_mut(&mut self, index: $ty) -> &mut UninitSlice {
                    let indexed = &mut self.0[index];
                    unsafe { &mut *(indexed as *mut [MaybeUninit<u8>] as *mut UninitSlice) }
                }
            }
       )*
    };
}

impl_index!(Range<usize>, RangeFull, RangeFrom<usize>);
