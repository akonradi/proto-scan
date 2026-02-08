use std::ops::{BitOrAssign, Shl};

use crate::DecodeError;
use crate::read::Read;

pub use parse::{LengthDelimited, ParseEvent, ParseEventReader, parse};
pub(crate) use tag::{Tag, WireType};
pub use wire_type::{I32, I64, ScalarWireType, Varint};

mod parse;
mod tag;
mod wire_type;

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Into)]
pub struct FieldNumber(u32);

trait ParseVarint:
    num_traits::Unsigned + num_traits::Zero + BitOrAssign + Shl<Output = Self> + From<u8>
{
    const MAX_BYTES: u8;
}

impl ParseVarint for u64 {
    const MAX_BYTES: u8 = 10;
}

impl ParseVarint for u32 {
    const MAX_BYTES: u8 = 5;
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
