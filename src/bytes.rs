use core::ops::{Deref, RangeBounds};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use core::{mem, ptr, slice};

use alloc::{
    alloc::{dealloc, Layout},
    borrow::Borrow,
    vec::Vec,
};

pub struct Bytes {
    /// A pointer to the underlying data
    ptr: *const u8,

    /// The len of the data
    len: usize,

    /// The counter to count the number of bytes with the same
    /// shared value alive
    data: AtomicPtr<()>,

    /// The virtual table to clone and drop this object
    vtable: &'static Vtable,
}

pub struct Vtable {
    pub(crate) clone: unsafe fn(&AtomicPtr<()>, *const u8, usize) -> Bytes,
    pub(crate) drop: unsafe fn(&mut AtomicPtr<()>, *const u8, usize),
}

// === Bytes ===

impl Bytes {
    const EMPTY: &[u8] = &[];

    #[inline]
    pub fn new() -> Bytes {
        Bytes::from_static(Bytes::EMPTY)
    }

    #[inline]
    pub const fn from_static(src: &'static [u8]) -> Bytes {
        Bytes {
            ptr: src.as_ptr(),
            len: src.len(),
            data: AtomicPtr::new(ptr::null_mut()),
            vtable: &STATIC_VTABLE,
        }
    }

    /// Return the len of the inner bytes buffer
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// let bytes = Bytes::new();
    ///
    /// assert!(bytes.is_empty());
    /// ```
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return true if the inner byte buffer is empty
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// let bytes = Bytes::new();
    ///
    /// assert!(bytes.is_empty());
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn copy_from_slice(src: &[u8]) -> Bytes {
        src.to_vec().into()
    }

    /// Retrieve the byte at the given index
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// let bytes = Bytes::from_static(b"toto");
    ///
    /// assert_eq!(bytes.get(1), b'o');
    /// ```
    ///
    /// # Préconditions
    ///
    /// `index` <= `self.len()`
    /// `index` >= 0
    ///
    /// # Panics
    ///
    /// If the preconditions are not met than this function will panic
    ///
    /// ```should_panic
    /// # use bytes::Bytes;
    /// let bytes = Bytes::new();
    ///
    /// bytes.get(1);
    /// ```
    pub fn get(&self, index: usize) -> u8 {
        assert!(
            (0..self.len).contains(&index),
            "index out of bound: bound ({}..{}) don't contains index ({})",
            0,
            self.len,
            index
        );

        let offset = unsafe { &self.ptr.add(index) };
        unsafe { offset.read() }
    }

    /// Get a subslice of the `Bytes` object
    /// This create a cloned `Bytes` object with the given subslice
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// let bytes = Bytes::from_static(b"toto toto");
    /// let slice = bytes.slice(..4);
    ///
    /// assert_eq!(slice.as_slice(), b"toto");
    /// assert_eq!(bytes.as_slice(), b"toto toto");
    /// ```
    ///
    /// # Panics
    ///
    /// This panics if there is an invalid range given e.g if the start is superior to the end
    /// or if the end is superior to the len of the `Bytes`
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Bytes {
        use core::ops::Bound::*;

        let len = self.len;

        let start = match range.start_bound() {
            Included(&start) => start,
            Excluded(&start) => start + 1,
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(&end) => end + 1,
            Excluded(&end) => end,
            Unbounded => 0,
        };

        assert!(
            start <= end,
            "invalid bounds: start ({}) > end ({})",
            start,
            end
        );

        assert!(end <= len, "invalid bounds: end out of bounds ({})", end);

        // If start == end we don't care about slicing the good ptr range
        // we just return an empty value
        if start == end {
            return Bytes::new();
        }

        let mut slice = self.clone();

        slice.len = end - start;
        slice.ptr = unsafe { slice.ptr.add(start) };

        slice
    }

    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let mut bytes = Bytes::from(b"hello world" as &[u8]);
    /// let b = bytes.split_off(5);
    ///
    /// assert_eq!(&bytes[..], b"hello");
    /// assert_eq!(&b[..], b" world");
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic if `at` > `self.len()`
    pub fn split_off(&mut self, at: usize) -> Bytes {
        assert!(
            at <= self.len,
            "index out of bounds: at ({}) > len ({})",
            at,
            self.len
        );

        let mut ret = self.clone();

        self.len = at;

        unsafe { ret.inc_start(at) };

        ret
    }

    ///
    /// # Example
    ///
    /// ```
    /// use bytes::Bytes;
    ///
    /// let mut bytes = Bytes::from(b"hello world" as &[u8]);
    /// let b = bytes.split_to(5);
    ///
    /// assert_eq!(&bytes[..], b" world");
    /// assert_eq!(&b[..], b"hello");
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic if `at` > `self.len()`
    pub fn split_to(&mut self, at: usize) -> Bytes {
        assert!(
            at <= self.len,
            "index out of bounds: at ({}) > len ({})",
            at,
            self.len
        );

        let mut ret = self.clone();

        unsafe { self.inc_start(at) };

        ret.len = at;
        ret
    }

    #[inline]
    unsafe fn inc_start(&mut self, inc: usize) {
        assert!(inc <= self.len());

        self.len -= inc;
        self.ptr = self.ptr.add(inc)
    }

    /// Retrive the inner bytes as a slice
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// let bytes = Bytes::from_static(b"toto");
    ///
    /// assert_eq!(bytes.as_slice(), b"toto");
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Clone for Bytes {
    fn clone(&self) -> Self {
        unsafe { (self.vtable.clone)(&self.data, self.ptr, self.len) }
    }
}

impl Drop for Bytes {
    fn drop(&mut self) {
        unsafe { (self.vtable.drop)(&mut self.data, self.ptr, self.len) }
    }
}

impl Default for Bytes {
    #[inline]
    fn default() -> Bytes {
        Bytes::new()
    }
}

// === AsRef, Borrow and Deref

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for Bytes {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<'a> IntoIterator for &'a Bytes {
    type Item = &'a u8;
    type IntoIter = slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

// === PartialEq, PartialOrd and Eq

// ** Bytes **

impl PartialEq<Bytes> for Bytes {
    fn eq(&self, other: &Bytes) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialOrd<Bytes> for Bytes {
    fn partial_cmp(&self, other: &Bytes) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl Eq for Bytes {}

// ** [u8] **

impl PartialEq<[u8]> for Bytes {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice() == other
    }
}

impl PartialOrd<[u8]> for Bytes {
    fn partial_cmp(&self, other: &[u8]) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other)
    }
}

impl PartialEq<Bytes> for [u8] {
    fn eq(&self, other: &Bytes) -> bool {
        self == other.as_slice()
    }
}

impl PartialOrd<Bytes> for [u8] {
    fn partial_cmp(&self, other: &Bytes) -> Option<std::cmp::Ordering> {
        self.partial_cmp(other.as_slice())
    }
}

// === From ===

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        let mut value = value;
        let len = value.len();
        let cap = value.capacity();
        let ptr = value.as_mut_ptr();

        // Avoid allocating new memory if possible
        if len == cap {
            return Bytes::from(value.into_boxed_slice());
        }

        let shared = Box::new(Shared {
            buf: ptr,
            cap,
            ref_cnt: AtomicUsize::new(1),
        });

        mem::forget(value);
        let shared = Box::into_raw(shared);

        Bytes {
            ptr,
            len,
            data: AtomicPtr::new(shared.cast()),
            vtable: &SHARED_VTABLE,
        }
    }
}

impl From<Box<[u8]>> for Bytes {
    fn from(value: Box<[u8]>) -> Self {
        // `Box` doesn't allocate memory for empty slices so we don't care about it
        if value.is_empty() {
            return Bytes::new();
        }

        let len = value.len();
        let ptr = Box::into_raw(value) as *mut u8;

        if ptr as usize & KIND_MASK == 0 {
            // We set the kind of the ptr to `KIND_UNSHARED` so that it can be shared
            // later on
            let data = map_ptr(ptr, |p| p | KIND_UNSHARED);
            Bytes {
                ptr,
                len,
                data: AtomicPtr::new(data.cast()),
                vtable: &PROMOTABLE_EVEN_VTABLE,
            }
        } else {
            Bytes {
                ptr,
                len,
                data: AtomicPtr::new(ptr.cast()),
                vtable: &PROMOTABLE_ODD_VTABLE,
            }
        }
    }
}

impl From<String> for Bytes {
    fn from(value: String) -> Self {
        Bytes::from(value.as_bytes().to_vec())
    }
}

impl From<&'static str> for Bytes {
    #[inline]
    fn from(value: &'static str) -> Self {
        Bytes::from_static(value.as_bytes())
    }
}

impl From<&'static [u8]> for Bytes {
    #[inline]
    fn from(value: &'static [u8]) -> Self {
        Bytes::from_static(value)
    }
}

// === Vtables ===
// === Static vtable ===

const STATIC_VTABLE: Vtable = Vtable {
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

// === Promotable vtable ===
// This is used to create `Bytes` from data already on the heap
// It avoids changing the data location if there is only one object
// using this data but it changes the location whenever the `Bytes` object is cloned

// Mask used to determine if a values needs to be promoted to a shared `Bytes`
const KIND_UNSHARED: usize = 0x1;
const KIND_SHARED: usize = 0x0;
const KIND_MASK: usize = 0x1;

const PROMOTABLE_ODD_VTABLE: Vtable = Vtable {
    clone: promotable_odd_clone,
    drop: promotable_odd_drop,
};

unsafe fn promotable_odd_clone(data: &AtomicPtr<()>, ptr: *const u8, len: usize) -> Bytes {
    let shared = data.load(Ordering::Relaxed);
    let kind = shared as usize & KIND_MASK;

    if kind == KIND_SHARED {
        shallow_clone_arc(shared.cast(), ptr, len)
    } else {
        debug_assert_eq!(kind, KIND_UNSHARED);
        shallow_clone_vec(data, shared, shared.cast(), ptr, len)
    }
}

unsafe fn promotable_odd_drop(data: &mut AtomicPtr<()>, ptr: *const u8, len: usize) {
    let data = data.get_mut();
    let shared = *data;
    let kind = shared as usize & KIND_MASK;

    if kind == KIND_SHARED {
        release_shared(shared.cast())
    } else {
        debug_assert_eq!(kind, KIND_UNSHARED);
        free_boxed_slice(shared.cast(), ptr, len)
    }
}

const PROMOTABLE_EVEN_VTABLE: Vtable = Vtable {
    clone: promotable_even_clone,
    drop: promotable_even_drop,
};

unsafe fn promotable_even_clone(data: &AtomicPtr<()>, ptr: *const u8, len: usize) -> Bytes {
    let shared = data.load(Ordering::Relaxed);
    let kind = shared as usize & KIND_MASK;

    if kind == KIND_SHARED {
        shallow_clone_arc(shared.cast(), ptr, len)
    } else {
        debug_assert_eq!(kind, KIND_UNSHARED);
        let buf = map_ptr(shared.cast(), |p| p & !KIND_MASK);
        shallow_clone_vec(data, shared, buf, ptr, len)
    }
}

unsafe fn promotable_even_drop(data: &mut AtomicPtr<()>, ptr: *const u8, len: usize) {
    let data = data.get_mut();
    let shared = *data;
    let kind = shared as usize & KIND_MASK;

    if kind == KIND_SHARED {
        release_shared(shared.cast())
    } else {
        debug_assert_eq!(kind, KIND_UNSHARED);
        let buf = map_ptr(shared.cast(), |p| p & !KIND_MASK);
        free_boxed_slice(buf, ptr, len)
    }
}

// === Shared vtable ===

const SHARED_VTABLE: Vtable = Vtable {
    clone: shared_clone,
    drop: shared_drop,
};

unsafe fn shared_clone(data: &AtomicPtr<()>, ptr: *const u8, len: usize) -> Bytes {
    let shared = data.load(Ordering::Relaxed);
    shallow_clone_arc(shared.cast(), ptr, len)
}

unsafe fn shared_drop(data: &mut AtomicPtr<()>, _: *const u8, _: usize) {
    let shared: *mut Shared = data.get_mut().cast();
    release_shared(shared)
}

unsafe fn shallow_clone_arc(shared: *mut Shared, ptr: *const u8, len: usize) -> Bytes {
    (*shared).ref_cnt.fetch_add(1, Ordering::Release);

    Bytes {
        ptr,
        len,
        data: AtomicPtr::new(shared.cast()),
        vtable: &SHARED_VTABLE,
    }
}

unsafe fn shallow_clone_vec(
    atom: &AtomicPtr<()>,
    ptr: *const (),
    buf: *mut u8,
    offset: *const u8,
    len: usize,
) -> Bytes {
    let shared = Box::new(Shared {
        buf,
        cap: (offset as usize - buf as usize) + len,
        ref_cnt: AtomicUsize::new(2),
    });

    let shared = Box::into_raw(shared);

    // Verif that the pointer is aligned
    // This is ensured by the `Box` API so this assert should not fail
    debug_assert_eq!(
        shared as usize & KIND_MASK,
        KIND_SHARED,
        "internal Box<Shared> should have an aligned pointer"
    );

    match atom.compare_exchange(ptr as _, shared.cast(), Ordering::AcqRel, Ordering::Acquire) {
        Ok(actual) => {
            debug_assert_eq!(actual as usize, ptr as usize);

            // Exchange was successful so we can return the new `Bytes` value
            Bytes {
                ptr: offset,
                len,
                data: AtomicPtr::new(shared.cast()),
                vtable: &SHARED_VTABLE,
            }
        }
        Err(actual) => {
            // The exchange was made by an other thread so we acquire the value
            // created by this other thread and we clone it into a new `Bytes` object

            // Forget the shared object we just allocated to create the new `Bytes` object
            let shared: Box<Shared> = Box::from_raw(actual as _);
            mem::forget(*shared);

            // Create an Arc copy of the `Bytes` object using the acquired new shared value
            shallow_clone_arc(actual.cast(), offset, len)
        }
    }
}

unsafe fn release_shared(shared: *mut Shared) {
    // If this is diffetent from 1 than we don't need to drop the value
    if (*shared).ref_cnt.fetch_sub(1, Ordering::Release) != 1 {
        return;
    }

    // Else we need to drop the underlying value
    drop(Box::from_raw(shared))
}

unsafe fn free_boxed_slice(buf: *mut u8, offset: *const u8, len: usize) {
    let cap = (offset as usize - buf as usize) + len;
    // TODO:
    // Safety: ?value
    dealloc(buf, Layout::from_size_align_unchecked(cap, 1))
}

struct Shared {
    buf: *mut u8,
    cap: usize,
    ref_cnt: AtomicUsize,
}

// Verify that the |Shared` struct size is divisible by 2 because we want to use the LSB has a flag.
const _: [(); 0 - mem::size_of::<Shared>() % 2] = [];

impl Drop for Shared {
    fn drop(&mut self) {
        unsafe { dealloc(self.buf, Layout::from_size_align(self.cap, 1).unwrap()) }
    }
}

// === Handfull functions to manipulate pointers ===

fn map_ptr<F>(ptr: *mut u8, f: F) -> *mut u8
where
    F: FnOnce(usize) -> usize,
{
    let old_ptr = ptr as usize;
    let new_ptr = f(old_ptr);
    new_ptr as *mut u8
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_iter {
        ($bytes:literal) => {
            let bytes = Bytes::from_static($bytes);
            assert_iter!(bytes => $bytes);
        };
        ($bytes:ident => $lit:literal) => {
            let mut iter = $lit.into_iter().enumerate();

            while let Some((index, byte)) = iter.next() {
                assert_eq!($bytes.get(index), *byte);
            }
        }
    }

    #[test]
    fn static_bytes() {
        assert_iter!(b"this is a static bytes");
        assert_iter!(b"another static bytes");
    }

    #[test]
    fn static_clone() {
        let bytes = Bytes::from_static(b"a static byte");
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
        assert_iter!(bytes => b"a static byte");
        assert_iter!(clone => b"a static byte");
    }

    #[test]
    fn shared_vec_clone() {
        let bytes = Bytes::from(b"toto".to_vec());
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
        assert_iter!(bytes => b"toto");
        assert_iter!(clone => b"toto");
    }

    #[test]
    fn shared_box_clone() {
        let boxed = b"toto".to_vec().into_boxed_slice();
        let bytes = Bytes::from(boxed);
        let clone = bytes.clone();

        assert_eq!(bytes.ptr, clone.ptr);
        assert_iter!(bytes => b"toto");
        assert_iter!(clone => b"toto");
    }

    #[test]
    fn copy_from_slice() {
        let bytes = Bytes::copy_from_slice(b"toto");

        assert_iter!(bytes => b"toto");
    }

    #[test]
    fn index() {
        let bytes = Bytes::from_static(b"this is a very long long bytes slice");

        assert_eq!(b"this", &bytes[..4]);
        assert_eq!(b"very long long bytes", &bytes[10..30]);
        assert_eq!(b"this is a very long long bytes slice", &bytes[..]);
    }

    #[test]
    fn slice() {
        let bytes = Bytes::from_static(b"this is a very long long bytes slice");

        let slice = bytes.slice(10..30);

        assert_eq!(b"very long long bytes", &slice[..]);
    }
}
