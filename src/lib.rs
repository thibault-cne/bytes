extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod buf;
mod bytes;
mod fmt;

pub use crate::bytes::Bytes;

pub use crate::buf::{Buf, BufMut};
