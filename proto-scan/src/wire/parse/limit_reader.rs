use crate::decode_error::DecodeVarintError;
use crate::read::{Read, ReadBytesError, ReadTypes};
use crate::wire::parse::event_reader::EventRead;
use crate::wire::{VARINT_MAX_BYTES, parse_base128_varint, varint_encoded_length};

pub(super) struct LimitReader<R> {
    inner: R,
    remaining: u32,
}

impl<R> LimitReader<R> {
    pub(super) fn new(inner: R, remaining: u32) -> Self {
        Self { inner, remaining }
    }

    pub(super) fn remaining(&self) -> u32 {
        self.remaining
    }
}

impl<R: Read> Read for LimitReader<R> {
    type ReadTypes = R::ReadTypes;

    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        let Self { inner, remaining } = self;
        if *remaining >= VARINT_MAX_BYTES.into() {
            let value = inner.read_varint()?;
            let consumed = varint_encoded_length(value);
            self.remaining -= u32::from(consumed);
            Ok(value)
        } else {
            let (value, _consumed) = parse_base128_varint(core::iter::from_fn(|| {
                Some(self.read(1).and_then(|buf| {
                    buf.as_ref()
                        .first()
                        .copied()
                        .ok_or(ReadBytesError::UnexpectedEnd)
                }))
            }))
            .map_err(|e| match e {
                DecodeVarintError::Read(ReadBytesError::UnexpectedEnd) => {
                    DecodeVarintError::UnexpectedEnd
                }
                DecodeVarintError::Read(ReadBytesError::Read(r)) => DecodeVarintError::Read(r),
                DecodeVarintError::InvalidVarint => DecodeVarintError::InvalidVarint,
                DecodeVarintError::UnexpectedEnd => DecodeVarintError::UnexpectedEnd,
            })?;
            Ok(value)
        }
    }

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        let Self { inner, remaining } = self;
        if bytes > *remaining {
            return Err(ReadBytesError::UnexpectedEnd);
        }
        let bytes = bytes.min(*remaining);
        let r = inner.read(bytes)?;
        *remaining -= bytes;
        Ok(r)
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
        let skipped = self.inner.skip(bytes)?;
        self.remaining -= skipped;
        Ok(skipped)
    }
}

impl<R: Read> EventRead for LimitReader<R> {
    type RealReader = R;
    fn skip_direct(
        &mut self,
        bytes: u32,
    ) -> Result<(), crate::DecodeError<<Self::ReadTypes as ReadTypes>::Error>> {
        let _ = self.inner.skip(bytes)?;
        Ok(())
    }

    fn take_as_reader(&mut self, bytes: u32) -> LimitReader<&mut Self::RealReader> {
        self.remaining -= bytes;
        LimitReader::new(&mut self.inner, bytes)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_from_empty() {
        let mut bytes = [1, 2, 3, 4].as_slice();
        let mut empty = LimitReader::new(&mut bytes, 0);
        assert_eq!(empty.read(1), Err(ReadBytesError::UnexpectedEnd));
    }

    #[test]
    fn short_inner() {
        let bytes = [1, 2, 3, 4].as_slice();
        let reader = &mut &bytes[..];
        let mut empty = LimitReader::new(reader, 5);
        assert_eq!(empty.read(5), Err(ReadBytesError::UnexpectedEnd));
    }

    #[test]
    fn read_prefix() {
        let bytes = [1, 2, 3, 4].as_slice();
        let mut empty = LimitReader::new(bytes, 3);
        assert_eq!(empty.read(2), Ok(&bytes[..2]));
    }

    #[test]
    fn read_more_than_remaining() {
        let bytes = [1, 2, 3, 4].as_slice();
        let mut empty = LimitReader::new(bytes, 3);
        assert_eq!(empty.read(5), Err(ReadBytesError::UnexpectedEnd));
    }
}
