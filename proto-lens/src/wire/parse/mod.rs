use std::fmt::Debug;
use std::num::NonZeroU32;

use either::Either;

use crate::DecodeError;
use crate::read::{Read, ReadError, ReadTypes};
use crate::wire::{FieldNumber, ScalarField, ScalarWireType};

use event_reader::EventReader;
use length_delimited::LengthDelimitedImpl;
use limit_reader::LimitReader;
use scalar_iter::ScalarIter;

mod event_reader;
mod length_delimited;
mod limit_reader;
mod scalar_iter;

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
    EventReader {
        inner: Either::Left(r),
        do_before: DoBeforeNext::DoNothing,
    }
}

#[derive(Debug, Default)]
enum DoBeforeNext {
    #[default]
    DoNothing,
    Skip(NonZeroU32),
    Error,
}

#[cfg(test)]
mod test {
    use crate::wire::serialize_base128_varint;
    use crate::wire::{FieldNumber, ScalarField, Tag, Varint, WireType};

    use super::*;

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
