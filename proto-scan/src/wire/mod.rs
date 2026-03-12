//! The low-level event interface uses [`ParseEventReader`] to read protobuf
//! wire format tags from a [`Read`] implementation.
//! 
//! ```
//! # use proto_scan::*;
//! use wire::ParseEventReader;
//! fn read_a<R: read::Read>(
//!     r: R,
//! ) -> Result<Option<i64>, DecodeError<<R::ReadTypes as read::ReadError>::Error>> {
//!     // From the protobuf documentation encoding guide.
//!     // message Test1 {
//!     //   int64 a = 1;
//!     // }
//!     let mut reader = wire::parse(r);
//!     let mut found_a = None;
//!     while let Some(event) = reader.next() {
//!         let (field_number, event) = event?;
//!         match event {
//!             wire::ParseEvent::Numeric(s) if field_number == 1 => match s {
//!                 wire::NumericField::Varint(v) => {
//!                     found_a = Some(
//!                         // cast bits according to protobuf encoding format
//!                         v as i64,
//!                     )
//!                 }
//!                 wire::NumericField::I64(_) | wire::NumericField::I32(_) => found_a = None,
//!             },
//!             wire::ParseEvent::Numeric(_)
//!             | wire::ParseEvent::Group(_)
//!             | wire::ParseEvent::LengthDelimited(_) => {}
//!         }
//!     }
//!     Ok(found_a)
//! }
//!
//! fn main() {
//!     // From the protobuf documentation encoding guide, this is a Test message
//!     // with a = 150.
//!     const INPUT: &[u8] = &[0x08, 0x96, 0x01];
//!
//!     assert_eq!(read_a(&mut &INPUT[..]), Ok(Some(150)))
//! }
//! ```
//!
//! In this example, the `read_a` method scans the provided input for a tag with
//! a varint wire type and field number 1. The last one found, if any, is
//! returned. Any other tag types or fields are ignored. The input is a `&mut &[u8]`
//! which has an implementation of [`Read`].
use core::ops::{BitAnd, BitOrAssign, Shl, Shr, ShrAssign};

use crate::DecodeError;
use crate::read::{Read, ReadError};

pub use field_number::{FieldNumber, InvalidFieldNumber};
pub use group_op::GroupOp;
pub use numeric_field::NumericField;
pub use parse::{LengthDelimited, ParseEvent, ParseEventReader, parse};
pub(crate) use tag::{Tag, WireType};
pub use wire_type::{I32, I64, NumericWireType, Varint, WrongWireType};

mod field_number;
mod group_op;
mod numeric_field;
mod parse;
mod tag;
mod wire_type;

trait ParseVarint:
    num_traits::Unsigned
    + num_traits::Zero
    + BitOrAssign
    + Shl<Output = Self>
    + From<u8>
    + Shr<Output = Self>
    + ShrAssign
    + BitAnd
    + PartialEq
{
    const MAX_BYTES: u8;

    #[cfg(test)]
    fn low_byte(&self) -> u8;
}

impl ParseVarint for u64 {
    const MAX_BYTES: u8 = 10;
    #[cfg(test)]
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

impl ParseVarint for u32 {
    const MAX_BYTES: u8 = 5;
    #[cfg(test)]
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

fn parse_base128_varint<R: Read, V: ParseVarint>(
    r: &mut R,
) -> Result<V, DecodeError<<R::ReadTypes as ReadError>::Error>> {
    let mut value = V::zero();
    for i in 0..V::MAX_BYTES {
        let byte = r.read(1).map_err(DecodeError::Read)?;
        let byte = *byte.as_ref().first().ok_or(DecodeError::UnexpectedEnd)?;
        let (byte, continue_flag) = (V::from(byte & !0x80), (byte & 0x80));

        value |= byte << (i * 7).into();
        if continue_flag == 0 {
            return Ok(value);
        }
    }
    Err(DecodeError::UnterminatedVarint)
}

#[cfg(test)]
fn serialize_base128_varint<V: ParseVarint>(mut value: V) -> arrayvec::ArrayVec<u8, 10> {
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
        const VALUES: [u32; 9] = [0, 1, 8, 127, 128, 255, 256, 1848593, u32::MAX];

        for value in VALUES {
            let serialized = serialize_base128_varint(value);
            let deserialized = parse_base128_varint::<_, u32>(&mut &serialized[..]);
            assert_eq!(
                deserialized,
                Ok(value),
                "{value} serialized as {serialized:?}"
            );
        }
    }
}
