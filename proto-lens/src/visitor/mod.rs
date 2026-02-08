use std::marker::PhantomData;

use crate::DecodeError;
use crate::read::Read;
use crate::wire::{I32, I64, ScalarWireType, Tag, Varint, WireType};

mod build;
pub use build::Builder;

pub trait Visitor {
    fn on_scalar(&mut self, field_number: u32, field: ScalarField);

    fn on_length_delimited<'s>(&'s mut self, handler: impl LengthDelimited + 's);

    fn on_group_begin(&mut self, field_number: u32);

    fn on_group_end(&mut self, field_number: u32);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarField {
    Varint(u64),
    I64(u64),
    I32(u32),
}

pub trait LengthDelimited {
    type ReadBuffer: AsRef<[u8]>;
    type ReadError: std::error::Error + 'static;

    fn len(&self) -> u32;

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Self::ReadError>>>;

    fn as_bytes(self) -> Result<Self::ReadBuffer, DecodeError<Self::ReadError>>;

    fn visit_as_message(self, visitor: impl Visitor);
}

pub fn visit_message<R: Read>(
    mut r: R,
    mut visitor: impl Visitor,
) -> Result<(), DecodeError<R::Error>> {
    loop {
        let mut tag_reader = CountReader {
            inner: &mut r,
            count: 0,
        };

        let tag = match Tag::read_from(&mut tag_reader) {
            Ok(tag) => tag,
            Err(DecodeError::UnexpectedEnd) if tag_reader.count == 0 => return Ok(()),
            Err(e) => return Err(e),
        };
        let Tag {
            wire_type,
            field_number,
        } = tag;

        match wire_type {
            WireType::Varint => {
                let value = Varint::read_from(&mut r)?;
                visitor.on_scalar(field_number, ScalarField::Varint(value));
            }
            WireType::I64 => {
                let value = I64::read_from(&mut r)?;
                visitor.on_scalar(field_number, ScalarField::I64(value));
            }
            WireType::I32 => {
                let value = I32::read_from(&mut r)?;
                visitor.on_scalar(field_number, ScalarField::I32(value));
            }
            WireType::Sgroup => visitor.on_group_begin(field_number),
            WireType::Egroup => visitor.on_group_end(field_number),
            WireType::LengthDelimited => {
                let length = Varint::read_from(&mut r)?;
                let length = u32::try_from(length)
                    .map_err(|_| DecodeError::<R::Error>::TooLargeLengthDelimited(length))?;

                let mut decode_result = Ok(());
                let mut reader = LimitReader {
                    inner: &mut r,
                    remaining: length,
                };

                visitor.on_length_delimited(LengthDelimitedImpl {
                    reader: &mut reader,
                    decode_result: &mut decode_result,
                });

                let () = decode_result?;
                let LimitReader {
                    inner: _,
                    remaining,
                } = reader;

                if remaining != 0 {
                    r.skip(remaining).map_err(DecodeError::Read)?;
                }
            }
        }
    }
}

struct LengthDelimitedImpl<'a, R: Read> {
    reader: &'a mut LimitReader<R>,
    decode_result: &'a mut Result<(), DecodeError<R::Error>>,
}

impl<'a, R: Read<Error: std::error::Error>> LengthDelimited for LengthDelimitedImpl<'a, R> {
    type ReadBuffer = R::Buffer;
    type ReadError = R::Error;

    fn len(&self) -> u32 {
        self.reader.remaining
    }

    fn as_bytes(self) -> Result<R::Buffer, DecodeError<R::Error>> {
        let bytes_len = self.reader.remaining;
        let bytes = self.reader.read(bytes_len).map_err(DecodeError::Read)?;

        if bytes.as_ref().len() != bytes_len as usize {
            return Err(DecodeError::UnexpectedEnd);
        }
        Ok(bytes)
    }

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<R::Error>>> {
        ScalarIter {
            reader: self.reader,
            _wire_type: PhantomData::<W>,
        }
    }

    fn visit_as_message(self, visitor: impl Visitor) {
        *self.decode_result = visit_message(self.reader, visitor).map_err(Into::into)
    }
}

struct ScalarIter<'a, R, W> {
    reader: &'a mut LimitReader<R>,
    _wire_type: PhantomData<W>,
}

struct LimitReader<R> {
    inner: R,
    remaining: u32,
}

impl<R: Read> Read for LimitReader<R> {
    type Buffer = R::Buffer;
    type Error = R::Error;

    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let r = self.inner.read(bytes)?;
        self.remaining = self.remaining - bytes;
        Ok(r)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        let skipped = self.inner.skip(bytes)?;
        self.remaining = self.remaining - skipped;
        Ok(skipped)
    }
}

struct CountReader<R> {
    inner: R,
    count: usize,
}

impl<R: Read> Read for CountReader<R> {
    type Buffer = R::Buffer;
    type Error = R::Error;

    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let r = self.inner.read(bytes)?;
        self.count += r.as_ref().len();
        Ok(r)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        let r = self.inner.skip(bytes)?;
        self.count += r as usize;
        Ok(r)
    }
}

impl<R: Read, W: ScalarWireType> Iterator for ScalarIter<'_, R, W> {
    type Item = Result<W::Repr, DecodeError<R::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader.remaining == 0 {
            return None;
        }

        Some(W::read_from(&mut self.reader).map_err(Into::into))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.remaining;
        let (min, max) = W::BYTE_LEN.into_inner();

        (
            (remaining / u32::from(max)).try_into().unwrap_or(0),
            (remaining.div_ceil(min.into())).try_into().ok(),
        )
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn extract_single_string() {
        let input = [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67];

        let mut extracted = None;

        let visitor = Builder::new(&mut extracted)
            .set_on_length_delimited(|extracted, delimited| {
                assert_matches!(
                    extracted.replace(delimited.as_bytes().expect("can read")),
                    None
                );
            })
            .build();

        let result = visit_message(&mut input.as_slice(), visitor);
        assert_matches!(result, Ok(()));
        assert_eq!(extracted, Some("testing".to_string().into()))
    }
}
