use crate::read::{Read, ReadError, ReadTypes};

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
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error> {
        let Self { inner, remaining } = self;
        let b = bytes.min(*remaining);
        let bytes = b;
        let r = inner.read(bytes)?;
        *remaining -= bytes;
        Ok(r)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error> {
        let skipped = self.inner.skip(bytes)?;
        self.remaining -= skipped;
        Ok(skipped)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_from_empty() {
        let mut bytes = [1, 2, 3, 4].as_slice();
        let mut empty = LimitReader::new(&mut bytes, 0);
        assert_eq!(empty.read(1), Ok([].as_slice()));
    }

    #[test]
    fn short_inner() {
        let bytes = [1, 2, 3, 4].as_slice();
        let reader = &mut &bytes[..];
        let mut empty = LimitReader::new(reader, 5);
        assert_eq!(empty.read(5), Ok(bytes));
    }

    #[test]
    fn read_more_than_remaining() {
        let bytes = [1, 2, 3, 4].as_slice();
        let mut empty = LimitReader::new(bytes, 3);
        assert_eq!(empty.read(5), Ok(&bytes[..3]));
    }
}
