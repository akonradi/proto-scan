use core::convert::Infallible;
use core::hash::Hash;
use core::ops::Deref;
use core::str::Utf8Error;

use either::Either;

mod bounds_only;
pub(crate) mod count_reader;
pub use bounds_only::BoundsOnlyReadTypes;

use crate::decode_error::DecodeVarintError;
use crate::wire::{parse_base128_varint, varint_bytes_chunk};

#[derive(Debug, PartialEq, derive_more::From)]
pub enum ReadBytesError<R> {
    #[from]
    Read(R),
    UnexpectedEnd,
}

pub trait ReadTypes {
    type Error: core::error::Error + 'static;
    type Buffer: ReadBuffer;
}

pub trait ReadBuffer: AsRef<[u8]> + Default + Eq + Hash {
    type String: Deref<Target = str> + Default + Eq + Hash;
    fn into_string(self) -> Result<Self::String, core::str::Utf8Error>;
}

pub trait Read {
    type ReadTypes: ReadTypes;

    /// Reads the next protobuf varint from the stream.
    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>>;

    /// Reads the given number of bytes.
    ///
    /// If fewer than the requested number of bytes are available before the
    /// stream ends, an error should be returned. Otherwise, the returned buffer
    /// must contain the requested number of bytes.
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    >;

    /// Returns the number of bytes skipped.
    ///
    /// If fewer than the requested number of bytes is available to be skipped,
    /// this method must return an error.
    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>>;
}

impl ReadTypes for &[u8] {
    type Error = Infallible;
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

impl Read for &[u8] {
    type ReadTypes = Self;

    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        let (value, consumed) = if let Some(bytes) = self.first_chunk() {
            parse_base128_varint(varint_bytes_chunk(bytes))
        } else {
            parse_base128_varint(self.iter().cloned().map(Ok))
        }?;
        *self = &self[consumed.into()..];
        Ok(value)
    }

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        let (bytes, after) = match self.split_at_checked(bytes as usize) {
            Some(split) => split,
            None => return Err(ReadBytesError::UnexpectedEnd),
        };

        *self = after;
        Ok(bytes)
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
        self.read(bytes)
            .map(|bytes| bytes.len().try_into().unwrap_or(u32::MAX))
    }
}

impl<R: ReadTypes> ReadTypes for &mut R {
    type Error = R::Error;
    type Buffer = R::Buffer;
}

impl<R: Read> Read for &mut R {
    type ReadTypes = R::ReadTypes;
    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        (*self).read_varint()
    }

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        (*self).read(bytes)
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
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
    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        self.as_mut()
            .map_either(|r| r.read_varint(), |r| r.read_varint())
            .into_inner()
    }

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        self.as_mut()
            .map_either(|r| r.read(bytes), |r| r.read(bytes))
            .into_inner()
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
        self.as_mut()
            .map_either(|r| r.skip(bytes), |r| r.skip(bytes))
            .into_inner()
    }
}

/// Implementation of [`Read`] that wraps a [`std::io::Read`] impl.
#[cfg(feature = "std")]
pub struct IoRead<R>(R);

#[cfg(feature = "std")]
impl<R> IoRead<R> {
    pub fn new(read: R) -> Self {
        Self(read)
    }
}

#[cfg(feature = "std")]
impl<R: std::io::Read> IoRead<std::io::BufReader<R>> {
    pub fn new_buffered(read: R) -> Self {
        Self(std::io::BufReader::new(read))
    }
}

#[cfg(feature = "std")]
impl<R: std::io::Read> ReadTypes for IoRead<R> {
    type Error = std::io::Error;
    type Buffer = Vec<u8>;
}

#[cfg(feature = "std")]
impl<R: std::io::BufRead + std::io::Seek> Read for IoRead<R> {
    type ReadTypes = Self;

    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        let buf = self.0.fill_buf()?;
        if let Some(bytes) = buf.first_chunk() {
            let (value, consumed) = parse_base128_varint(varint_bytes_chunk(bytes))?;
            self.0.consume(consumed.into());
            return Ok(value);
        }

        parse_base128_varint(std::iter::from_fn(|| {
            let mut buf = [0];
            Some(self.0.read_exact(&mut buf).map(|()| buf[0]))
        }))
        .map(|(value, _consumed)| value)
    }

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        let mut buffer = Vec::with_capacity(bytes.try_into().unwrap_or(usize::MAX));
        self.0.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
        let skipped = self.0.seek(std::io::SeekFrom::Current(bytes.into()))?;
        if skipped != bytes.into() {
            return Err(ReadBytesError::UnexpectedEnd);
        }
        Ok(bytes)
    }
}

#[cfg(feature = "std")]
impl ReadBuffer for Vec<u8> {
    type String = String;

    fn into_string(self) -> Result<Self::String, core::str::Utf8Error> {
        String::from_utf8(self).map_err(|e| e.utf8_error())
    }
}
