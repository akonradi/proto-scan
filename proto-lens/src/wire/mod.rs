use std::ops::{BitAnd, BitOrAssign, Shl, Shr, ShrAssign};

use crate::DecodeError;
use crate::read::Read;

pub use parse::{LengthDelimited, ParseEvent, ParseEventReader, parse};
pub(crate) use tag::{Tag, WireType};
pub use wire_type::{I32, I64, ScalarWireType, Varint};

mod parse;
mod tag;
mod wire_type;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::Into)]
pub struct FieldNumber(u32);

#[derive(Copy, Clone, Debug)]
pub struct InvalidFieldNumber(pub u32);

impl TryFrom<u32> for FieldNumber {
    type Error = InvalidFieldNumber;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value < 1 << 29 {
            return Ok(Self(value));
        }
        Err(InvalidFieldNumber(value))
    }
}

impl PartialEq<u32> for FieldNumber {
    fn eq(&self, other: &u32) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<FieldNumber> for u32 {
    fn eq(&self, other: &FieldNumber) -> bool {
        self.eq(&other.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarField {
    Varint(u64),
    I64(u64),
    I32(u32),
}

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

    fn low_byte(&self) -> u8;
}

impl ParseVarint for u64 {
    const MAX_BYTES: u8 = 10;
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

impl ParseVarint for u32 {
    const MAX_BYTES: u8 = 5;
    fn low_byte(&self) -> u8 {
        *self as u8
    }
}

fn parse_base128_varint<R: Read, V: ParseVarint>(r: &mut R) -> Result<V, DecodeError<R::Error>> {
    let mut value = V::zero();
    for i in 0..V::MAX_BYTES {
        let byte = r.read(1).map_err(DecodeError::Read)?;
        let byte = *byte
            .as_ref()
            .first()
            .ok_or(DecodeError::<R::Error>::UnexpectedEnd)?;
        let (byte, continue_flag) = (V::from(byte & !0x80), (byte & 0x80));

        value |= byte << (i * 7).into();
        if continue_flag == 0 {
            return Ok(value);
        }
    }
    Err(DecodeError::UnterminatedVarint)
}

fn serialize_base128_varint<V: ParseVarint>(mut value: V) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(V::MAX_BYTES.into());

    loop {
        let mut v: u8 = value.low_byte() & 0x7f ;
        value >>= 7.into();

        if value == V::zero() {
            bytes.push(v);
            break;
        }

        v |= 0x80;
        bytes.push(v);
    }

    bytes.into_boxed_slice()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
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
