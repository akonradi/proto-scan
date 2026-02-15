use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use assert_matches::debug_assert_matches;
use either::Either;

use crate::DecodeError;
use crate::read::{Read, ReadError, ReadTypes};
use crate::wire::{FieldNumber, I32, I64, ScalarField, ScalarWireType, Tag, Varint, WireType};

/// Accessor for the contents of a length-delimited field.
///
/// Length-delimited fields are used to encode several different types of values
/// - raw byte sequences (`repeated byte` or `string` fields)
/// - embedded messages
/// - packed repeated scalar fields
///
/// This trait allows interpreting the contents of length-delimited field as at
/// most one of those representations.
pub trait LengthDelimited: ReadTypes {
    /// Returns the number of bytes in the field.
    fn len(&self) -> u32;

    /// Interprets the contents of a field as packed values.
    ///
    /// Consumes the contents of the field and returns an iterator that, on `next()`,
    /// returns the next value or an error if it cannot be decoded. If the
    /// iterator is dropped before it is exhausted, the remaining values and/or
    /// read errors are dropped.
    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Self::Error>>>;

    /// Reads the contents of the delimited field as bytes.
    fn as_bytes(self) -> Result<Self::Buffer, DecodeError<Self::Error>>;

    /// Consumes the contents of the field as a sequence of [`ParseEvent`]s in
    /// the form of a [`ParseEventReader`].
    fn as_events(self) -> impl ParseEventReader<Error = Self::Error>;
}

/// Protobuf parser that interprets a message as a sequence of events.
///
/// This acts as a lending iterator, where the [`ParseEvent`] returned by
/// [`Self::next`] can borrow from `self`.
pub trait ParseEventReader: ReadError {
    /// Advances the parse state and returns the next event, an error, or `None`
    /// if the underlying source is exhausted.
    fn next(
        &mut self,
    ) -> Option<
        Result<
            (
                FieldNumber,
                ParseEvent<impl LengthDelimited<Error = <Self as ReadError>::Error>>,
            ),
            DecodeError<<Self as ReadError>::Error>,
        >,
    >;
}

/// An event returned by [`ParseEventReader::next`].
#[derive(Debug)]
pub enum ParseEvent<L> {
    /// A scalar field was encountered.
    Scalar(ScalarField),
    /// An group was opened.
    StartGroup,
    /// A group was closed.
    EndGroup,
    /// A length-delimited field was encountered.
    LengthDelimited(L),
}

pub fn parse<'a, R: Read + 'a>(r: R) -> impl ParseEventReader<Error = R::Error> + 'a {
    Impl {
        inner: Either::Left(r),
        do_before: DoBeforeNext::DoNothing,
    }
}

struct Impl<'a, R> {
    inner: Either<R, LengthDelimitedImpl<'a, R>>,
    do_before: DoBeforeNext,
}

#[derive(Debug, Default)]
enum DoBeforeNext {
    #[default]
    DoNothing,
    Skip(NonZeroU32),
    Error,
}

impl<R: ReadError> ReadError for Impl<'_, R> {
    type Error = R::Error;
}

impl<'a, R: Read> ParseEventReader for Impl<'a, R> {
    fn next(
        &mut self,
    ) -> Option<
        Result<
            (
                FieldNumber,
                ParseEvent<impl LengthDelimited<Error = Self::Error>>,
            ),
            DecodeError<Self::Error>,
        >,
    > {
        let Self { inner, do_before } = self;

        let reader = inner;

        match std::mem::take(do_before) {
            DoBeforeNext::Skip(to_skip) => {
                if let Err(e) = reader.skip(to_skip.get()) {
                    return Some(Err(DecodeError::Read(e)));
                }
            }
            DoBeforeNext::Error => return Some(Err(DecodeError::UnexpectedEnd)),
            DoBeforeNext::DoNothing => {}
        }

        let mut tag_reader = CountReader {
            inner: reader,
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

        Some(
            match wire_type {
                WireType::Varint => Varint::read_from(&mut self.inner)
                    .map(|value| ParseEvent::Scalar(ScalarField::Varint(value))),
                WireType::I64 => I64::read_from(&mut &mut self.inner)
                    .map(|value| ParseEvent::Scalar(ScalarField::I64(value))),
                WireType::I32 => I32::read_from(&mut &mut self.inner)
                    .map(|value| ParseEvent::Scalar(ScalarField::I32(value))),
                WireType::Sgroup => Ok(ParseEvent::StartGroup),
                WireType::Egroup => Ok(ParseEvent::EndGroup),
                WireType::LengthDelimited => {
                    let to_skip = match (|| {
                        let length = Varint::read_from(&mut self.inner)?;
                        u32::try_from(length)
                            .map_err(|_| DecodeError::<R::Error>::TooLargeLengthDelimited(length))
                    })() {
                        Err(e) => return Some(Err(e)),
                        Ok(l) => l,
                    };

                    Ok(ParseEvent::LengthDelimited(LengthDelimitedImpl {
                        reader: LimitReader {
                            inner: &mut self.inner,
                            remaining: to_skip,
                        },
                        write_back_to: do_before,
                    }))
                }
            }
            .map(|field| (field_number, field)),
        )
    }
}

struct LengthDelimitedImpl<'a, R> {
    reader: LimitReader<&'a mut R>,
    write_back_to: &'a mut DoBeforeNext,
}

impl<R: Read> ReadError for LengthDelimitedImpl<'_, R> {
    type Error = R::Error;
}

impl<R: Read> ReadTypes for LengthDelimitedImpl<'_, R> {
    type Buffer = R::Buffer;
}

impl<R: Read> Read for LengthDelimitedImpl<'_, R> {
    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        match self.reader.read(bytes) {
            Ok(b) => Ok(b),
            Err(e) => {
                *self.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        }
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, Self::Error> {
        match self.reader.skip(bytes) {
            Ok(b) => Ok(b),
            Err(e) => {
                *self.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        }
    }
}

impl<'a, R: Read<Error: std::error::Error>> LengthDelimited for LengthDelimitedImpl<'a, R> {
    fn len(&self) -> u32 {
        self.reader.remaining
    }

    fn as_bytes(mut self) -> Result<R::Buffer, DecodeError<R::Error>> {
        let remaining = self.reader.remaining;
        let bytes = self.reader.read(remaining).map_err(DecodeError::Read)?;
        if bytes.as_ref().len() != remaining as usize {
            *self.write_back_to = DoBeforeNext::Error;
            return Err(DecodeError::UnexpectedEnd);
        }
        Ok(bytes)
    }

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<R::Error>>> {
        ScalarIter {
            inner: self,
            _wire_type: PhantomData::<W>,
        }
    }

    fn as_events(self) -> impl ParseEventReader<Error = Self::Error> {
        Impl {
            inner: Either::Right(self),
            do_before: DoBeforeNext::DoNothing,
        }
    }
}

impl<'a, R> Drop for LengthDelimitedImpl<'a, R> {
    fn drop(&mut self) {
        let Self {
            reader,
            write_back_to,
        } = self;
        if let Some(remaining) = NonZeroU32::new(reader.remaining) {
            debug_assert_matches!(write_back_to, DoBeforeNext::DoNothing | DoBeforeNext::Error);
            match write_back_to {
                DoBeforeNext::DoNothing | DoBeforeNext::Skip(_) => {
                    **write_back_to = DoBeforeNext::Skip(remaining)
                }
                DoBeforeNext::Error => {}
            }
        }
    }
}

struct ScalarIter<'a, R, W> {
    inner: LengthDelimitedImpl<'a, R>,
    _wire_type: PhantomData<W>,
}

struct LimitReader<R> {
    inner: R,
    remaining: u32,
}

impl<R: ReadError> ReadError for LimitReader<R> {
    type Error = R::Error;
}

impl<R: ReadTypes> ReadTypes for LimitReader<R> {
    type Buffer = R::Buffer;
}

impl<R: Read> Read for LimitReader<R> {
    fn read(&mut self, bytes: u32) -> Result<Self::Buffer, Self::Error> {
        let Self { inner, remaining } = self;
        let b = bytes.min(*remaining);
        let bytes = b;
        let r = inner.read(bytes)?;
        *remaining = *remaining - bytes;
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

impl<R: ReadError> ReadError for CountReader<R> {
    type Error = R::Error;
}

impl<R: ReadTypes> ReadTypes for CountReader<R> {
    type Buffer = R::Buffer;
}

impl<R: Read> Read for CountReader<R> {
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

impl<'a, R: Read, W: ScalarWireType> Iterator for ScalarIter<'a, R, W> {
    type Item = Result<W::Repr, DecodeError<R::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let Self { inner, _wire_type } = self;
        if inner.reader.remaining == 0 {
            return None;
        }

        Some(match W::read_from(inner) {
            Ok(r) => Ok(r),
            Err(e) => {
                *self.inner.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.inner.reader.remaining;
        let (min, max) = W::BYTE_LEN.into_inner();

        (
            (remaining / u32::from(max)).try_into().unwrap_or(0),
            (remaining.div_ceil(min.into())).try_into().ok(),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::wire::serialize_base128_varint;

    use super::*;

    mod limit_reader {
        use super::*;
        #[test]
        fn read_from_empty() {
            let mut empty = LimitReader {
                inner: &mut [1, 2, 3, 4].as_slice(),
                remaining: 0,
            };
            assert_eq!(empty.read(1), Ok([].as_slice()));
        }

        #[test]
        fn short_inner() {
            let bytes = [1, 2, 3, 4].as_slice();
            let mut empty = LimitReader {
                inner: &mut &bytes[..],
                remaining: 5,
            };
            assert_eq!(empty.read(5), Ok(bytes));
        }

        #[test]
        fn read_more_than_remaining() {
            let bytes = [1, 2, 3, 4].as_slice();
            let mut empty = LimitReader {
                inner: &mut &bytes[..],
                remaining: 3,
            };
            assert_eq!(empty.read(5), Ok(&bytes[..3]));
        }
    }

    #[test]
    fn extract_single_string() {
        // message { field string = 2; }
        let input = [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67];
        let mut input = input.as_slice();

        let mut reader = parse(&mut input);

        let event = reader.next().unwrap();
        let length_delimited = match event.unwrap() {
            (FieldNumber(2), ParseEvent::LengthDelimited(l)) => l,
            _ => panic!("invalid"),
        };

        assert_eq!(length_delimited.as_bytes().unwrap().as_ref(), b"testing");
    }

    #[test]
    fn drop_packed_iter_advances() {
        let input = [
            Tag {
                field_number: FieldNumber(1),
                wire_type: WireType::LengthDelimited,
            }
            .serialized(),
            serialize_base128_varint(3u32),
            vec![0, 1, 2].into_boxed_slice(),
            Tag {
                field_number: FieldNumber(2),
                wire_type: WireType::I32,
            }
            .serialized(),
            ScalarField::I32(0x04030201).serialize(),
        ]
        .concat();

        let mut read = &input[..];
        let mut parse = parse(&mut read);

        let mut iter = match parse.next().unwrap().unwrap() {
            (field_number, ParseEvent::LengthDelimited(value)) => {
                assert_eq!(field_number, 1);
                value.as_packed::<Varint>()
            }
            (
                field_number,
                ParseEvent::Scalar(_) | ParseEvent::StartGroup | ParseEvent::EndGroup,
            ) => {
                panic!("wrong event; field = {field_number:?}")
            }
        };
        assert_eq!(iter.next(), Some(Ok(0)));
        // If the iterator is dropped without exhausting it, the remaining
        // values are still skipped.
        drop(iter);

        let next = parse.next().unwrap().unwrap();
        match next {
            (field_number, ParseEvent::Scalar(scalar_field)) => {
                assert_eq!(field_number, 2);
                assert_eq!(scalar_field, ScalarField::I32(0x04030201))
            }
            (
                field_number,
                ParseEvent::StartGroup | ParseEvent::EndGroup | ParseEvent::LengthDelimited(_),
            ) => {
                panic!("wrong event field {field_number:?}")
            }
        }
    }

    #[test]
    fn drop_embedded_message_advance() {
        let input = [
            Tag {
                field_number: FieldNumber(1),
                wire_type: WireType::LengthDelimited,
            }
            .serialized(),
            serialize_base128_varint(4u32),
            // Embedded message begin
            Tag {
                field_number: FieldNumber(6),
                wire_type: WireType::Varint,
            }
            .serialized(),
            serialize_base128_varint(1u32),
            Tag {
                field_number: FieldNumber(7),
                wire_type: WireType::Varint,
            }
            .serialized(),
            serialize_base128_varint(2u32),
            // Embedded message end
            Tag {
                field_number: FieldNumber(2),
                wire_type: WireType::I32,
            }
            .serialized(),
            ScalarField::I32(0x04030201).serialize(),
        ]
        .concat();

        let mut read = &input[..];
        let mut parse = parse(&mut read);

        let embedded = match parse.next().unwrap().unwrap() {
            (field_number, ParseEvent::LengthDelimited(value)) => {
                assert_eq!(field_number, 1);
                value
            }
            (
                field_number,
                ParseEvent::Scalar(_) | ParseEvent::StartGroup | ParseEvent::EndGroup,
            ) => {
                panic!("wrong event; field = {field_number:?}")
            }
        };
        {
            let mut embedded_events = embedded.as_events();
            match embedded_events.next().unwrap().unwrap() {
                (FieldNumber(6), ParseEvent::Scalar(ScalarField::Varint(1))) => {}
                (
                    field_number,
                    ParseEvent::Scalar(_)
                    | ParseEvent::StartGroup
                    | ParseEvent::EndGroup
                    | ParseEvent::LengthDelimited(_),
                ) => {
                    panic!("wrong event field {field_number:?}")
                }
            }

            // If the iterator is dropped without exhausting it, the remaining
            // values are still skipped.
            drop(embedded_events);
        }

        let next = parse.next().unwrap().unwrap();
        match next {
            (field_number, ParseEvent::Scalar(scalar_field)) => {
                assert_eq!(field_number, 2);
                assert_eq!(scalar_field, ScalarField::I32(0x04030201))
            }
            (
                field_number,
                ParseEvent::StartGroup | ParseEvent::EndGroup | ParseEvent::LengthDelimited(_),
            ) => {
                panic!("wrong event field {field_number:?}")
            }
        }
    }
}
