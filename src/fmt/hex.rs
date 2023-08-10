use core::fmt::{LowerHex, UpperHex};

use super::BytesFmt;
use crate::Bytes;

impl<'a> LowerHex for BytesFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{:2x}", b)?;
        }

        Ok(())
    }
}

impl<'a> UpperHex for BytesFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{:2X}", b)?;
        }

        Ok(())
    }
}

macro_rules! hex_impl {
    ($($trait:ident => $ty:ty),*) => {
       $(
           impl $trait for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    $trait::fmt(&BytesFmt(self.as_ref()), f)
                }

           }
       )*
    };
}

hex_impl!(
    LowerHex => Bytes,
    UpperHex => Bytes
);
