use std::slice;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use std::alloc::{dealloc, Layout};
use std::mem;

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
    pub fn from_static(src: &'static [u8]) -> Bytes {
        Bytes {
            ptr: src.as_ptr(),
            len: src.len(),
            cpt: AtomicPtr::new(&mut ()),
            vtable: &STATIC_VTABLE,
        }
    }

    pub fn get(&self, index: usize) -> u8 {
        // TODO: panic if index if greater than self.len
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

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        let mut value = value;
        let len = value.len();
        let cap = value.capacity();
        let ptr = value.as_mut_ptr();

        let shared = Box::new(Shared {
            buf: ptr,
            cap,
            ref_count: AtomicUsize::new(1),
        });

        mem::forget(value);
        let shared = Box::into_raw(shared);

        Bytes {
            ptr,
            len,
            cpt: AtomicPtr::new(shared as _),
            vtable: &SHARED_VTABLE,
        }
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

// === Shared vtable ===
// This is used to create a shared bytes object
// from a vector or a boxed u8 slice

static SHARED_VTABLE: VTable = VTable {
    clone: shared_clone,
    drop: shared_drop,
};

unsafe fn shared_clone(data: &AtomicPtr<()>, ptr: *const u8, len: usize) -> Bytes {
    let shared = data.load(Ordering::Relaxed);
    shallow_clone(shared as _, ptr, len)
}

unsafe fn shared_drop(data: &mut AtomicPtr<()>, _: *const u8, _: usize) {
    let shared: *mut Shared = data.get_mut().cast();
    release_shared(shared)
}

unsafe fn shallow_clone(shared: *mut Shared, ptr: *const u8, len: usize) -> Bytes {
    (*shared).ref_count.fetch_add(1, Ordering::Release);

    Bytes {
        ptr,
        len,
        cpt: AtomicPtr::new(shared as _),
        vtable: &SHARED_VTABLE,
    }
}

unsafe fn release_shared(shared: *mut Shared) {
    // If this is diffetent from 1 than we don't need to drop the value
    if (*shared).ref_count.fetch_sub(1, Ordering::Release) != 1 {
        return;
    }

    // Else we need to drop the underlying value
    drop(Box::from_raw(shared))
}

struct Shared {
    buf: *mut u8,
    cap: usize,
    ref_count: AtomicUsize,
}

impl Drop for Shared {
    fn drop(&mut self) {
        unsafe { dealloc(self.buf, Layout::from_size_align(self.cap, 1).unwrap()) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_iter {
        ($bytes:literal) => {
            let bytes = Bytes::from_static($bytes);
            let mut iter = $bytes.into_iter().enumerate();

            while let Some((index, byte)) = iter.next() {
                assert_eq!(bytes.get(index), *byte);
            }
        };
    }

    #[test]
    fn static_bytes() {
        assert_iter!(b"this is a static bytes");
    }

    #[test]
    fn static_clone() {
        let bytes = Bytes::from_static(b"a static byte");
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
    }

    #[test]
    fn shared_clone() {
        let bytes = Bytes::from("toto".as_bytes().to_vec());
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
    }
}
