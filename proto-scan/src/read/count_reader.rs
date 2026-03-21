use crate::read::{Read, ReadError, ReadTypes};
use crate::wire::varint_encoded_length;

pub(crate) struct CountReader<R> {
    inner: R,
    count: usize,
}

impl<R: ReadError> ReadError for CountReader<R> {
    type Error = R::Error;
}

impl<R: ReadTypes> ReadTypes for CountReader<R> {
    type Buffer = R::Buffer;
}

impl<R: Read> Read for CountReader<R> {
    type ReadTypes = R::ReadTypes;
    fn read_varint(
        &mut self,
    ) -> Result<u64, crate::decode_error::DecodeVarintError<<Self::ReadTypes as ReadError>::Error>>
    {
        let r = self.inner.read_varint()?;
        self.count += usize::from(varint_encoded_length(r));
        Ok(r)
    }
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        super::ReadBytesError<<Self::ReadTypes as ReadError>::Error>,
    > {
        let r = self.inner.read(bytes)?;
        self.count += r.as_ref().len();
        Ok(r)
    }

    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, super::ReadBytesError<<Self::ReadTypes as ReadError>::Error>> {
        let r = self.inner.skip(bytes)?;
        self.count += r as usize;
        Ok(r)
    }
}

impl<R> CountReader<R> {
    pub(crate) fn new(reader: R) -> Self {
        CountReader {
            inner: reader,
            count: 0,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.count
    }
}
