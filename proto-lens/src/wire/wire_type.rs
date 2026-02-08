use std::convert::Infallible as Never;

use crate::DecodeError;
use crate::read::Read;

pub trait ScalarWireType {
    type Repr;
    const BYTE_LEN: std::ops::RangeInclusive<u8>;

    fn read_from<R: Read>(r: &mut R) -> Result<Self::Repr, DecodeError<R::Error>>;
}

pub struct Varint(Never);

pub struct I64(Never);

pub struct I32(Never);

mod sealed {
    #[allow(dead_code)]
    pub trait Sealed {}

    impl Sealed for super::Varint {}
    impl Sealed for super::I64 {}
    impl Sealed for super::I32 {}
}


impl ScalarWireType for Varint {
    type Repr = u64;
    const BYTE_LEN: std::ops::RangeInclusive<u8> = 1..=10;

    fn read_from<R: Read>(r: &mut R) -> Result<Self::Repr, DecodeError<R::Error>> {
        super::parse_base128_varint(r)
    }
}

pub(crate) fn read_fixed_from<const N: usize, R: Read, V>(
    r: &mut R,
) -> Result<V, DecodeError<R::Error>>
where
    V: num_traits::FromBytes<Bytes = [u8; N]>,
{
    let bytes = r.read(N as u32).map_err(DecodeError::Read)?;

    let bytes = bytes
        .as_ref()
        .as_array()
        .ok_or(DecodeError::<R::Error>::UnexpectedEnd)?;

    Ok(num_traits::FromBytes::from_le_bytes(bytes))
}

impl ScalarWireType for I64 {
    type Repr = u64;
    const BYTE_LEN: std::ops::RangeInclusive<u8> = 8..=8;

    fn read_from<R: Read>(r: &mut R) -> Result<Self::Repr, DecodeError<R::Error>> {
        read_fixed_from(r)
    }
}

impl ScalarWireType for I32 {
    type Repr = u32;
    const BYTE_LEN: std::ops::RangeInclusive<u8> = 4..=4;

    fn read_from<R: Read>(r: &mut R) -> Result<Self::Repr, DecodeError<R::Error>> {
        read_fixed_from(r)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    #[test]
    fn varint_decode() {
        let input = [0x96, 0x01];

        let result = Varint::read_from(&mut input.as_slice());
        assert_eq!(result, Ok(150))
    }

    #[test]
    fn i32_decode() {
        let input = [0x01, 0x02, 0x03, 0x04];

        let result = I32::read_from(&mut input.as_slice());

        assert_eq!(result, Ok(0x04030201))
    }
}
