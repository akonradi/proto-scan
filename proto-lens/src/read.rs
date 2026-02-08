use std::convert::Infallible as Never;

pub trait Read {
    type Buffer: AsRef<[u8]>;
    type Error: std::error::Error + 'static;

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

impl<'a> Read for &'a [u8] {
    type Buffer = Self;
    type Error = Never;

    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let (bytes, after) = match self.split_at_checked(bytes as usize) {
            Some(split) => split,
            None => return Ok(&[]),
        };

        *self = after;
        Ok(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        self.read(bytes)
            .map(|bytes| bytes.len().try_into().unwrap_or(u32::MAX))
    }
}

impl<R: Read> Read for &mut R {
    type Buffer = R::Buffer;
    type Error = R::Error;

    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        (*self).read(bytes)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        (*self).skip(bytes)
    }
}
