use core::convert::Infallible as Never;

use crate::read::{ReadBuffer, ReadError, ReadTypes};

pub struct BoundsOnlyReadTypes(Never);

impl ReadError for BoundsOnlyReadTypes {
    type Error = Never;
}

impl ReadTypes for BoundsOnlyReadTypes {
    type Buffer = [u8; 0];
}

impl ReadBuffer for [u8; 0] {
    type String = &'static str;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        Ok("")
    }
}
