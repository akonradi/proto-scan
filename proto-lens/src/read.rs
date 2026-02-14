use std::convert::Infallible as Never;

pub trait ReadError {
    type Error: std::error::Error + 'static;
}

pub trait ReadTypes: ReadError {
    type Buffer: ReadBuffer;
}

pub trait ReadBuffer: AsRef<[u8]> {
    fn into_bytes(self) -> Box<[u8]>;
}

pub trait Read: ReadTypes {
    /// Reads the given number of bytes.
    ///
    /// If fewer than the requested number of bytes are available before the stream ends,
    /// the remaining bytes should be returned. Otherwise, the full number of
    /// requested bytes must be returned.
    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error>;

    /// Returns the number of bytes skipped.
    ///
    /// Implementations must only skip fewer than the number of bytes requested
    /// if the end of the stream is reached.
    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error>;
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
    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let (bytes, after) = match self.split_at_checked(bytes as usize) {
            Some(split) => split,
            None => return Ok(std::mem::take(self))
        };

        *self = after;
        Ok(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
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
    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        (*self).read(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        (*self).skip(bytes)
    }
}
