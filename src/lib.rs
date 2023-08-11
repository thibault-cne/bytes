extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod buf;
mod bytes;
mod bytes_str;
mod fmt;

pub use crate::bytes::Bytes;
pub use crate::bytes_str::BytesStr;

pub use crate::buf::{Buf, BufMut};
