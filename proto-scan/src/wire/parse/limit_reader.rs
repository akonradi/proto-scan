use crate::decode_error::DecodeVarintError;
use crate::read::{Read, ReadBytesError, ReadTypes};
use crate::wire::parse::event_reader::EventRead;
use crate::wire::{VARINT_MAX_BYTES, parse_base128_varint};

#[derive(Debug)]
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

impl<R: Read> LimitReader<R> {
    /// Decodes a varint one byte at a time, so a bulk read cannot overrun the limit.
    ///
    /// Deliberately not inlined: this is the cold half of [`Read::read_varint`],
    /// and inlining it there pushes that method past the inlining threshold.
    #[inline(never)]
    fn read_varint_near_limit(
        &mut self,
    ) -> Result<(u64, u8), DecodeVarintError<<<Self as Read>::ReadTypes as ReadTypes>::Error>> {
        parse_base128_varint(core::iter::from_fn(|| {
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
        })
    }
}

impl<R: Read> Read for LimitReader<R> {
    type ReadTypes = R::ReadTypes;

    #[inline]
    fn read_varint(
        &mut self,
    ) -> Result<(u64, u8), DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        let Self { inner, remaining } = self;
        if *remaining >= VARINT_MAX_BYTES.into() {
            let (value, consumed) = inner.read_varint()?;
            self.remaining -= u32::from(consumed);
            Ok((value, consumed))
        } else {
            // Fall back to an out-of-line call for the slow path.
            self.read_varint_near_limit()
        }
    }

    #[inline]
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

    #[inline]
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
