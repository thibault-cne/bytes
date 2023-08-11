extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod buf;
mod byte_str;
mod bytes;
mod fmt;

pub use crate::byte_str::BytesStr;
pub use crate::bytes::Bytes;

pub use crate::buf::{Buf, BufMut};
