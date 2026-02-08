use std::fmt::Debug;
use std::marker::PhantomData;

use crate::DecodeError;
use crate::read::Read;
use crate::wire::{FieldNumber, I32, I64, ScalarField, ScalarWireType, Tag, Varint, WireType};

pub trait LengthDelimited {
    type ReadBuffer: AsRef<[u8]>;
    type ReadError: std::error::Error + 'static;

    fn len(&self) -> u32;

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Self::ReadError>>>;

    fn as_bytes(self) -> Result<Self::ReadBuffer, DecodeError<Self::ReadError>>;

    fn as_events(self) -> impl ParseEventReader<ReadError = Self::ReadError>;
}

pub trait ParseEventReader {
    type ReadError: std::error::Error + 'static;

    fn next(
        &mut self,
    ) -> Option<
        Result<
            ParseEvent<impl LengthDelimited<ReadError = Self::ReadError>>,
            DecodeError<Self::ReadError>,
        >,
    >;
}

#[derive(Debug)]
pub enum ParseEvent<L> {
    Scalar(FieldNumber, ScalarField),
    StartGroup(FieldNumber),
    EndGroup(FieldNumber),
    LengthDelimited(FieldNumber, L),
}

pub fn parse<R: Read>(r: R) -> impl ParseEventReader<ReadError = R::Error> {
    Impl {
        reader: r,
        error: OwnedOrMut::Owned(false),
    }
}

enum OwnedOrMut<'a, T> {
    Owned(T),
    Mut(&'a mut T),
}

impl<T> AsRef<T> for OwnedOrMut<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            OwnedOrMut::Owned(t) => &t,
            OwnedOrMut::Mut(t) => &*t,
        }
    }
}

impl<T> AsMut<T> for OwnedOrMut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            OwnedOrMut::Owned(t) => t,
            OwnedOrMut::Mut(t) => t,
        }
    }
}

struct Impl<'a, R: Read> {
    reader: R,
    error: OwnedOrMut<'a, bool>,
}

impl<'a, R: Read> ParseEventReader for Impl<'a, R> {
    type ReadError = R::Error;
    fn next(
        &mut self,
    ) -> Option<
        Result<
            ParseEvent<impl LengthDelimited<ReadError = Self::ReadError>>,
            DecodeError<Self::ReadError>,
        >,
    > {
        if *self.error.as_ref() {
            return None;
        }

        let mut tag_reader = CountReader {
            inner: &mut self.reader,
            count: 0,
        };

        let tag = match Tag::read_from(&mut tag_reader) {
            Ok(tag) => tag,
            Err(DecodeError::UnexpectedEnd) if tag_reader.count == 0 => return None,
            Err(e) => return Some(Err(e)),
        };
        let Tag {
            wire_type,
            field_number,
        } = tag;
        let field_number = FieldNumber(field_number);

        Some(match wire_type {
            WireType::Varint => Varint::read_from(&mut &mut self.reader)
                .map(|value| ParseEvent::Scalar(field_number, ScalarField::Varint(value))),
            WireType::I64 => I64::read_from(&mut &mut self.reader)
                .map(|value| ParseEvent::Scalar(field_number, ScalarField::I64(value))),
            WireType::I32 => I32::read_from(&mut &mut self.reader)
                .map(|value| ParseEvent::Scalar(field_number, ScalarField::I32(value))),
            WireType::Sgroup => Ok(ParseEvent::StartGroup(field_number)),
            WireType::Egroup => Ok(ParseEvent::EndGroup(field_number)),
            WireType::LengthDelimited => {
                let length = match (|| {
                    let length = Varint::read_from(&mut self.reader)?;
                    u32::try_from(length)
                        .map_err(|_| DecodeError::<R::Error>::TooLargeLengthDelimited(length))
                })() {
                    Err(e) => return Some(Err(e)),
                    Ok(l) => l,
                };

                Ok(ParseEvent::LengthDelimited(
                    field_number,
                    LengthDelimitedImpl {
                        reader: Some(LimitReader {
                            inner: &mut self.reader,
                            remaining: length,
                        }),
                        error: OwnedOrMut::Mut(self.error.as_mut()),
                    },
                ))
            }
        })
    }
}

struct LengthDelimitedImpl<'a, R: Read> {
    reader: Option<LimitReader<&'a mut R>>,
    error: OwnedOrMut<'a, bool>,
}

impl<'a, R: Read> Drop for LengthDelimitedImpl<'a, R> {
    fn drop(&mut self) {
        let Self { reader, error } = self;
        let reader = reader.take().unwrap();
        let Ok(_) = reader.inner.skip(reader.remaining) else {
            *error.as_mut() = true;
            return;
        };
    }
}

impl<'a, R: Read<Error: std::error::Error>> LengthDelimited for LengthDelimitedImpl<'a, R> {
    type ReadBuffer = R::Buffer;
    type ReadError = R::Error;

    fn len(&self) -> u32 {
        self.reader.as_ref().unwrap().remaining
    }

    fn as_bytes(mut self) -> Result<R::Buffer, DecodeError<R::Error>> {
        let remaining = self.reader.as_ref().unwrap().remaining;
        let bytes = self.read(remaining).map_err(DecodeError::Read)?;
        if bytes.as_ref().len() != remaining as usize {
            return Err(DecodeError::UnexpectedEnd);
        }
        Ok(bytes)
    }

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<R::Error>>> {
        ScalarIter {
            reader: self,
            _wire_type: PhantomData::<W>,
        }
    }

    fn as_events(mut self) -> impl ParseEventReader<ReadError = Self::ReadError> {
        Impl {
            error: std::mem::replace(&mut self.error, OwnedOrMut::Owned(false)),
            reader: self,
        }
    }
}

impl<R: Read> Read for LengthDelimitedImpl<'_, R> {
    type Buffer = R::Buffer;
    type Error = R::Error;

    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let reader = self.reader.as_mut().unwrap();
        let buffer = reader.read(bytes)?;

        if buffer.as_ref().len() != bytes as usize {
            *self.error.as_mut() = true;
        }
        Ok(buffer)
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        self.reader.as_mut().unwrap().skip(bytes)
    }
}

struct ScalarIter<R, W> {
    reader: R,
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

impl<R: Read, W: ScalarWireType> Iterator for ScalarIter<LengthDelimitedImpl<'_, R>, W> {
    type Item = Result<W::Repr, DecodeError<R::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader.reader.as_ref().unwrap().remaining == 0 {
            return None;
        }

        Some(W::read_from(&mut self.reader).map_err(Into::into))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.reader.as_ref().unwrap().remaining;
        let (min, max) = W::BYTE_LEN.into_inner();

        (
            (remaining / u32::from(max)).try_into().unwrap_or(0),
            (remaining.div_ceil(min.into())).try_into().ok(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn extract_single_string() {
        // message { field string = 2; }
        let input = [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67];
        let mut input = input.as_slice();

        let mut reader = parse(&mut input);

        let event = reader.next().unwrap();
        let length_delimited = match event.unwrap() {
            ParseEvent::LengthDelimited(FieldNumber(2), l) => l,
            _ => panic!("invalid"),
        };

        assert_eq!(length_delimited.as_bytes().unwrap().as_ref(), b"testing");
    }
}
