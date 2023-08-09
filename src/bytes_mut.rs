use alloc::{sync::Arc, vec::Vec};

pub struct BytesMut {
    len: usize,
    cap: usize,
    _data: Arc<Vec<u8>>,
}

const MAX_ORIGINAL_CAPACITY: usize = 17;
const MIN_ORIGINAL_CAPACITY: usize = 10;

impl BytesMut {
    /// Create a new empty `BytesMut`
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::bytes_mut::BytesMut;
    /// let buf = BytesMut::new();
    ///
    /// assert_eq!(buf.len, 0);
    pub fn new() -> BytesMut {
        BytesMut::from_vec(Vec::new())
    }

    /// Retrieve the current len of the inner buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return true if the inner buffer has a len of 0
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Retrieve the capacity of the `BytesMut` object
    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub(crate) fn from_vec(vec: Vec<u8>) -> BytesMut {
        let cap = match vec.capacity() {
            c if c > MAX_ORIGINAL_CAPACITY => MAX_ORIGINAL_CAPACITY,
            c if c < MAX_ORIGINAL_CAPACITY => MIN_ORIGINAL_CAPACITY,
            c => c,
        };

        BytesMut {
            cap,
            len: 0,
            _data: Arc::new(Vec::with_capacity(cap)),
        }
    }
}

impl Default for BytesMut {
    fn default() -> Self {
        BytesMut::new()
    }
}
