use core::convert::Infallible as Never;

use crate::read::{ReadBuffer, ReadTypes};

pub type BoundsOnlyReadTypes = [u8; 0];

impl ReadBuffer for [u8; 0] {
    type String = &'static str;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        Ok("")
    }
}

impl ReadTypes for [u8; 0] {
    type Error = Never;
    type Buffer = [u8; 0];
}
