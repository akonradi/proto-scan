use core::fmt::Debug;
use core::num::NonZeroU32;

use either::Either;

use crate::DecodeError;
use crate::read::{Read, ReadError, ReadTypes};
use crate::wire::{FieldNumber, GroupOp, NumericField, NumericWireType};

use event_reader::EventReader;
use length_delimited::LengthDelimitedImpl;
use limit_reader::LimitReader;
use numeric_iter::NumericIter;

mod event_reader;
mod length_delimited;
mod limit_reader;
mod numeric_iter;

pub trait DelimitedTypes {
    type ReadTypes: ReadTypes;
}

/// Accessor for the contents of a length-delimited field.
///
/// Length-delimited fields are used to encode several different types of values
/// - raw byte sequences (`repeated byte` or `string` fields)
/// - embedded messages
/// - packed repeated numeric fields
///
/// This trait allows interpreting the contents of length-delimited field as at
/// most one of those representations.
pub trait LengthDelimited: DelimitedTypes {
    /// Returns the number of bytes in the field.
    fn len(&self) -> u32;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    type PackedIter<W: NumericWireType>: Iterator<
        Item = Result<W::Repr, DecodeError<<Self::ReadTypes as ReadError>::Error>>,
    >;

    /// Interprets the contents of a field as packed values.
    ///
    /// Consumes the contents of the field and returns an iterator that, on `next()`,
    /// returns the next value or an error if it cannot be decoded. If the
    /// iterator is dropped before it is exhausted, the remaining values and/or
    /// read errors are dropped.
    fn into_packed<W: NumericWireType>(self) -> Self::PackedIter<W>;

    /// Reads the contents of the delimited field as bytes.
    fn into_bytes(
        self,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        DecodeError<<Self::ReadTypes as ReadError>::Error>,
    >;

    /// Consumes the contents of the field as a sequence of [`ParseEvent`]s in
    /// the form of a [`ParseEventReader`].
    fn into_events(self) -> impl ParseEventReader<ReadTypes = Self::ReadTypes>;
}

/// Protobuf parser that interprets a message as a sequence of events.
///
/// This acts as a lending iterator, where the [`ParseEvent`] returned by
/// [`Self::next`] can borrow from `self`.
pub trait ParseEventReader {
    type ReadTypes: ReadTypes;
    /// Advances the parse state and returns the next event, an error, or `None`
    /// if the underlying source is exhausted.
    fn next(
        &mut self,
    ) -> Option<
        ParseEventReaderOutput<Self::ReadTypes, impl LengthDelimited<ReadTypes = Self::ReadTypes>>,
    >;
}

pub(crate) type ParseEventReaderOutput<RT, LD> =
    Result<(FieldNumber, ParseEvent<LD>), DecodeError<<RT as ReadError>::Error>>;

/// A tag and value parsed from a protobuf stream.
///
/// Each instance corresponds to a protobuf wire format tag and its
/// corresponding contents. Numeric values are accessible directly and
/// length-delimited contents are available via the [`LengthDelimited`] trait.
#[derive(Debug)]
pub enum ParseEvent<L> {
    /// A numeric field was encountered.
    Numeric(NumericField),
    /// An group was opened or closed.
    Group(GroupOp),
    /// A length-delimited field was encountered.
    ///
    /// Handling code can use the methods of the [`LengthDelimited`] trait to
    /// access the contents of the field.
    LengthDelimited(L),
}

pub fn parse<'a, R: Read + 'a>(r: R) -> impl ParseEventReader<ReadTypes = R::ReadTypes> + 'a {
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
    use crate::wire::{FieldNumber, NumericField, Tag, Varint, WireType, serialize_base128_varint};

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

        assert_eq!(length_delimited.into_bytes().unwrap().as_ref(), b"testing");
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
            [0, 1, 2].into_iter().collect(),
            Tag {
                field_number: FieldNumber(2),
                wire_type: WireType::I32,
            }
            .serialized(),
            NumericField::I32(0x04030201).serialize(),
        ]
        .concat();

        let mut read = &input[..];
        let mut parse = parse(&mut read);

        let mut iter = match parse.next().unwrap().unwrap() {
            (field_number, ParseEvent::LengthDelimited(value)) => {
                assert_eq!(field_number, 1);
                value.into_packed::<Varint>()
            }
            (field_number, ParseEvent::Numeric(_) | ParseEvent::Group(_)) => {
                panic!("wrong event; field = {field_number:?}")
            }
        };
        assert_eq!(iter.next(), Some(Ok(0)));
        // If the iterator is dropped without exhausting it, the remaining
        // values are still skipped.
        drop(iter);

        let next = parse.next().unwrap().unwrap();
        match next {
            (field_number, ParseEvent::Numeric(numeric_field)) => {
                assert_eq!(field_number, 2);
                assert_eq!(numeric_field, NumericField::I32(0x04030201))
            }
            (field_number, ParseEvent::Group(_) | ParseEvent::LengthDelimited(_)) => {
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
            NumericField::I32(0x04030201).serialize(),
        ]
        .concat();

        let mut read = &input[..];
        let mut parse = parse(&mut read);

        let embedded = match parse.next().unwrap().unwrap() {
            (field_number, ParseEvent::LengthDelimited(value)) => {
                assert_eq!(field_number, 1);
                value
            }
            (field_number, ParseEvent::Numeric(_) | ParseEvent::Group(_)) => {
                panic!("wrong event; field = {field_number:?}")
            }
        };
        {
            let mut embedded_events = embedded.into_events();
            match embedded_events.next().unwrap().unwrap() {
                (FieldNumber(6), ParseEvent::Numeric(NumericField::Varint(1))) => {}
                (
                    field_number,
                    ParseEvent::Numeric(_) | ParseEvent::Group(_) | ParseEvent::LengthDelimited(_),
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
            (field_number, ParseEvent::Numeric(numeric_field)) => {
                assert_eq!(field_number, 2);
                assert_eq!(numeric_field, NumericField::I32(0x04030201))
            }
            (field_number, ParseEvent::Group(_) | ParseEvent::LengthDelimited(_)) => {
                panic!("wrong event field {field_number:?}")
            }
        }
    }
}
