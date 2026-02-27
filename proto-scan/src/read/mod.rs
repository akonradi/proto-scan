use std::convert::Infallible as Never;

use either::Either;

pub(crate) mod count_reader;

pub trait ReadError {
    type Error: std::error::Error + 'static;
}

pub trait ReadTypes: ReadError {
    type Buffer: ReadBuffer;
}

pub trait ReadBuffer: AsRef<[u8]> {
    fn into_bytes(self) -> Box<[u8]>;
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

impl ReadError for &[u8] {
    type Error = Never;
}

impl<'a> ReadTypes for &'a [u8] {
    type Buffer = Self;
}

impl ReadBuffer for &[u8] {
    fn into_bytes(self) -> Box<[u8]> {
        self.to_owned().into_boxed_slice()
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
            None => return Ok(std::mem::take(self)),
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
