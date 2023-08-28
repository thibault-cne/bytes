extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod buf;
mod byte_str;
mod bytes;
mod bytes_mut;
mod fmt;
mod iter;

pub use crate::byte_str::ByteStr;
pub use crate::bytes::Bytes;
pub use crate::bytes_mut::BytesMut;

pub use crate::buf::{Buf, BufMut};
