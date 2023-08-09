extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod bytes;
mod bytes_mut;

pub use crate::bytes::Bytes;
pub use crate::bytes_mut::BytesMut;
