//! The low-level event interface treats the input stream as a sequence of events
//! corresponding to individual tags and their contents. The entry point is
//! the [`parse`] function, which returns a [`ParseEventReader`] implementation.
//!
//! The `ParseEventReader` is a lending iterator. On a call to
//! [`ParseEventReader::next`], it reads the next protobuf tag from the input stream,
//! decodes it, and then returns a corresponding [`ParseEvent`] that mutably
//! borrows the reader. The mutable borrow allows the [`ParseEvent::LengthDelimited`]
//! variant to hold a [`LengthDelimited`] implementation that can be used to
//! access (or skip over) the contents of a length-delimited field.
//!
//! To read a message in full, calling code must call `ParseEventReader::next`
//! until it returns `None`, signaling the end of the input. At any point,
//! calling code can drop the reader to avoid reading the rest of the input
//! stream (though, given the [merge semantics] of protocol buffers, this isn't
//! frequently useful).
//!
//! As an example, here is a method that scans for a protobuf int64 field and
//! returns its value, if found.
//! ```
//! # use proto_scan::*;
//! use wire::ParseEventReader;
//! fn read_int64<R: read::Read>(
//!     field_number: u32,
//!     r: R,
//! ) -> Result<Option<i64>, DecodeError<<R::ReadTypes as read::ReadTypes>::Error>> {
//!     let mut reader = wire::parse(r);
//!     let mut found = None;
//!     while let Some((number, event)) = reader.next().transpose()? {
//!         match event {
//!             wire::ParseEvent::Numeric(s) if number == field_number => match s {
//!                 wire::NumericField::Varint(v) => {
//!                     found = Some(
//!                         // cast bits according to protobuf encoding format
//!                         v as i64,
//!                     )
//!                 }
//!                 wire::NumericField::I64(_) | wire::NumericField::I32(_) => found = None,
//!             },
//!             wire::ParseEvent::Numeric(_)
//!             | wire::ParseEvent::Group(_)
//!             | wire::ParseEvent::LengthDelimited(_) => {}
//!         }
//!     }
//!     Ok(found)
//! }
//! # // Make sure the example actually works.
//! # fn main() {
//! #     // From the protobuf documentation encoding guide.
//! #     // message Test1 {
//! #     //   int64 a = 1;
//! #     // }
//! #     // This is a Test message with a = 150.
//! #     const INPUT: &[u8] = &[0x08, 0x96, 0x01];
//! #     assert_eq!(read_int64(1, &mut &INPUT[..]), Ok(Some(150)))
//! # }
//! ```
//!
//! [merge semantics]: https://protobuf.dev/programming-guides/encoding/#last-one-wins
use crate::decode_error::DecodeVarintError;

pub use field_number::{FieldNumber, InvalidFieldNumber};
pub use group_op::GroupOp;
pub use numeric_field::NumericField;
pub use parse::ParseCallbacks;
pub use parse::{DelimitedTypes, LengthDelimited, ParseEvent, ParseEventReader, parse};
pub(crate) use tag::{Tag, WireType};
pub use wire_type::{I32, I64, NumericWireType, Varint, WrongWireType};

mod field_number;
mod group_op;
mod numeric_field;
mod parse;
mod tag;
mod wire_type;

pub const VARINT_MAX_BYTES: u8 = 10;

pub fn parse_base128_varint<R>(
    bytes: impl IntoIterator<Item = Result<u8, R>>,
) -> Result<(u64, u8), DecodeVarintError<R>> {
    let mut value = 0u64;
    let mut bytes = bytes.into_iter();
    for i in 0..VARINT_MAX_BYTES {
        let byte = bytes.next().ok_or(DecodeVarintError::UnexpectedEnd)??;
        let (byte, continue_flag) = (byte & !0x80, byte & 0x80 != 0);

        if i != 0 && !continue_flag && byte == 0 {
            // Reject varints that were encoded with more than the necessary
            // number of bytes.
            return Err(DecodeVarintError::InvalidVarint);
        }

        value |= u64::from(byte) << (i * 7);
        if !continue_flag {
            return Ok((value, i + 1));
        }
    }

    Err(DecodeVarintError::InvalidVarint)
}

pub fn varint_bytes_chunk<E>(
    bytes: &[u8; VARINT_MAX_BYTES as usize],
) -> impl IntoIterator<Item = Result<u8, E>> {
    bytes.iter().cloned().map(Ok)
}

pub fn varint_encoded_length(value: u64) -> u8 {
    (u64::BITS - value.leading_zeros()) as u8 / 7 + 1
}

#[cfg(test)]
pub(crate) trait ParseVarint:
    num_traits::Num
    + core::ops::BitOrAssign
    + core::ops::Shl<Output = Self>
    + From<u8>
    + core::ops::Shr<Output = Self>
    + core::ops::ShrAssign
    + core::ops::BitAnd
{
    fn low_byte(&self) -> u8;
}

#[cfg(test)]
impl ParseVarint for u64 {
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

#[cfg(test)]
impl ParseVarint for u32 {
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

#[cfg(test)]
pub(crate) fn serialize_base128_varint<V: ParseVarint>(mut value: V) -> arrayvec::ArrayVec<u8, 10> {
    let mut bytes = arrayvec::ArrayVec::new();

    loop {
        let mut v: u8 = value.low_byte() & 0x7f;
        value >>= 7.into();

        if value == V::zero() {
            bytes.push(v);
            break;
        }

        v |= 0x80;
        bytes.push(v);
    }

    bytes
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn u32_inverse() {
        const VALUES: [u64; 9] = [0, 1, 8, 127, 128, 255, 256, 1848593, u32::MAX as u64];

        for value in VALUES {
            use std::convert::Infallible;

            let serialized = serialize_base128_varint(value);
            let deserialized =
                parse_base128_varint::<Infallible>(serialized.iter().cloned().map(Ok)).map(|v| v.0);
            assert_eq!(
                deserialized,
                Ok(value),
                "{value} serialized as {serialized:?}"
            );
        }
    }
}
