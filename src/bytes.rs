use std::slice;
use std::sync::atomic::AtomicPtr;

pub struct Bytes {
    /// A pointer to the underlying data
    ptr: *const u8,

    /// The len of the data
    len: usize,

    /// The counter to count the number of bytes with the same
    /// shared value alive
    cpt: AtomicPtr<()>,

    /// The virtual table to clone and drop this object
    vtable: &'static VTable,
}

pub struct VTable {
    pub(crate) clone: unsafe fn(&AtomicPtr<()>, *const u8, usize) -> Bytes,
    pub(crate) drop: unsafe fn(&mut AtomicPtr<()>, *const u8, usize),
}

// === Bytes ===

impl Bytes {
    #[inline]
    pub fn from_static(src: &'static [u8]) -> Bytes {
        Bytes {
            ptr: src.as_ptr(),
            len: src.len(),
            cpt: AtomicPtr::new(&mut ()),
            vtable: &STATIC_VTABLE,
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> u8 {
        let offset = unsafe { &self.ptr.add(index) };
        unsafe { offset.read() }
    }
}

impl Clone for Bytes {
    fn clone(&self) -> Self {
        unsafe { (self.vtable.clone)(&self.cpt, self.ptr, self.len) }
    }
}

impl Drop for Bytes {
    fn drop(&mut self) {
        unsafe { (self.vtable.drop)(&mut self.cpt, self.ptr, self.len) }
    }
}

// === VTables ===

pub static STATIC_VTABLE: VTable = VTable {
    clone: static_clone,
    drop: static_drop,
};

unsafe fn static_clone(_: &AtomicPtr<()>, ptr: *const u8, len: usize) -> Bytes {
    // Because the underlying value is static we don't care about
    // the reference counter
    let slice = slice::from_raw_parts(ptr, len);
    Bytes::from_static(slice)
}

unsafe fn static_drop(_: &mut AtomicPtr<()>, _: *const u8, _: usize) {
    // Nothing to do
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! indexes {
        ($bytes:literal, $($index:expr => $value:literal),*) => {
            let bytes = Bytes::from_static($bytes);

           $(
               assert_eq!(bytes.get($index), $value);
            )*
        };
    }

    #[test]
    fn static_bytes() {
        indexes!(
            b"this is a static bytes",
            0 => b't',
            1 => b'h',
            2 => b'i',
            3 => b's'
        );
    }

    #[test]
    fn static_clone() {
        let bytes = Bytes::from_static(b"a static byte");
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
    }
}
