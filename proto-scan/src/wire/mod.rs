use core::ops::{BitAnd, BitOrAssign, Shl, Shr, ShrAssign};

use crate::DecodeError;
use crate::read::{Read, ReadError};

pub use field_number::{FieldNumber, InvalidFieldNumber};
pub use group_op::GroupOp;
pub use numeric_field::NumericField;
pub use parse::{LengthDelimited, ParseEvent, ParseEventReader, parse};
pub(crate) use tag::{Tag, WireType};
pub use wire_type::{I32, I64, NumericWireType, Varint};

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
