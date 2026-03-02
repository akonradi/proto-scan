use core::convert::Infallible as Never;
use core::ops::Deref;
use core::str::Utf8Error;

use either::Either;

pub(crate) mod count_reader;

pub trait ReadError {
    type Error: core::error::Error + 'static;
}

pub trait ReadTypes: ReadError {
    type Buffer: ReadBuffer;
}

pub trait ReadBuffer: AsRef<[u8]> {
    type String: Deref<Target = str>;
    fn into_string(self) -> Result<Self::String, core::str::Utf8Error>;
}

pub trait Read {
    type ReadTypes: ReadTypes;
    /// Reads the given number of bytes.
    ///
    /// If fewer than the requested number of bytes are available before the stream ends,
    /// the remaining bytes should be returned. Otherwise, the full number of
    /// requested bytes must be returned.
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error>;

    /// Returns the number of bytes skipped.
    ///
    /// Implementations must only skip fewer than the number of bytes requested
    /// if the end of the stream is reached.
    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error>;
}

pub struct BoundsOnlyReadTypes(Never);

impl ReadError for BoundsOnlyReadTypes {
    type Error = Never;
}
impl ReadTypes for BoundsOnlyReadTypes {
    type Buffer = NeverBuffer;
}

impl ReadError for &[u8] {
    type Error = Never;
}

impl<'a> ReadTypes for &'a [u8] {
    type Buffer = Self;
}

impl<'a> ReadBuffer for &'a [u8] {
    type String = &'a str;
    fn into_string(self) -> Result<Self::String, Utf8Error>
    where
        Self: Sized,
    {
        core::str::from_utf8(self)
    }
}

impl<'a> Read for &'a [u8] {
    type ReadTypes = Self;
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error> {
        let (bytes, after) = match self.split_at_checked(bytes as usize) {
            Some(split) => split,
            None => return Ok(core::mem::take(self)),
        };

        *self = after;
        Ok(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error> {
        self.read(bytes)
            .map(|bytes| bytes.len().try_into().unwrap_or(u32::MAX))
    }
}

impl<R: ReadError> ReadError for &mut R {
    type Error = R::Error;
}

impl<R: ReadTypes> ReadTypes for &mut R {
    type Buffer = R::Buffer;
}

impl<R: Read> Read for &mut R {
    type ReadTypes = R::ReadTypes;
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error> {
        (*self).read(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error> {
        (*self).skip(bytes)
    }
}

#[cfg(feature = "bytes")]
impl ReadBuffer for ::bytes::Bytes {
    type String = bytes_utils::Str;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        bytes_utils::Str::try_from(self).map_err(|e| e.utf8_error())
    }
}

impl<R: Read, L: Read<ReadTypes = R::ReadTypes>> Read for Either<L, R> {
    type ReadTypes = R::ReadTypes;
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error> {
        self.as_mut()
            .map_either(|r| r.read(bytes), |r| r.read(bytes))
            .into_inner()
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error> {
        self.as_mut()
            .map_either(|r| r.skip(bytes), |r| r.skip(bytes))
            .into_inner()
    }
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
    type String = NeverBuffer;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        todo!()
    }
}
