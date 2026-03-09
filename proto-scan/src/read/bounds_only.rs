use core::convert::Infallible as Never;

use crate::read::{ReadBuffer, ReadError, ReadTypes};

pub struct BoundsOnlyReadTypes(Never);

impl ReadError for BoundsOnlyReadTypes {
    type Error = Never;
}
impl ReadTypes for BoundsOnlyReadTypes {
    type Buffer = NeverBuffer;
}

pub struct NeverBuffer(Never);

impl AsRef<[u8]> for NeverBuffer {
    fn as_ref(&self) -> &[u8] {
        match self.0 {}
    }
}

impl core::ops::Deref for NeverBuffer {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self.0 {}
    }
}

impl ReadBuffer for NeverBuffer {
    type String = &'static str;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        match self.0 {}
    }
}
