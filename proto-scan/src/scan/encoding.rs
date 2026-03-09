use core::convert::Infallible;
use core::marker::PhantomData;
use core::num::TryFromIntError;

use crate::wire::NumericWireType;

pub struct Varint<T>(PhantomData<T>);
pub struct Fixed<T>(PhantomData<T>);
pub struct ZigZag<T>(PhantomData<T>);

pub trait Encoding {
    type Wire: NumericWireType;
    type Repr: Copy;
    type Error: Into<VarintOutOfBounds>;

    fn decode(wire: <Self::Wire as NumericWireType>::Repr) -> Result<Self::Repr, Self::Error>;
}

pub struct VarintOutOfBounds;

impl From<Infallible> for VarintOutOfBounds {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl From<TryFromIntError> for VarintOutOfBounds {
    fn from(_: TryFromIntError) -> Self {
        VarintOutOfBounds
    }
}

fn zigzag_decode<U>(u: U) -> U
where
    U: num_traits::ConstOne + num_traits::ConstZero + num_traits::int::PrimInt,
{
    let sign = u & U::ONE;
    let xor = if sign == U::ZERO {
        U::ZERO
    } else {
        U::max_value()
    };
    let x = u ^ xor;
    ((x & U::ONE.not()) | sign).rotate_right(1)
}

impl Encoding for Varint<bool> {
    type Wire = super::Varint;

    type Repr = bool;
    type Error = VarintOutOfBounds;

    fn decode(wire: <Self::Wire as NumericWireType>::Repr) -> Result<bool, Self::Error> {
        match wire {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(VarintOutOfBounds),
        }
    }
}

macro_rules! impl_encoding {
    (Varint<ZigZag<$t:ty>>, $repr:ty) => {
        impl Encoding for Varint<ZigZag<$t>> {
            type Wire = super::Varint;

            type Repr = $t;
            type Error = VarintOutOfBounds;

            fn decode(wire: <Self::Wire as NumericWireType>::Repr) -> Result<$t, Self::Error> {
                let unzigged = zigzag_decode(<$repr>::try_from(wire)?);
                let bytes = unzigged.to_ne_bytes();
                Ok(<$t>::from_ne_bytes(bytes))
            }
        }
    };
    (Varint<$t:ty>, $repr:ty) => {
        impl Encoding for Varint<$t> {
            type Wire = super::Varint;

            type Repr = $t;
            type Error = VarintOutOfBounds;

            fn decode(wire: <Self::Wire as NumericWireType>::Repr) -> Result<$t, Self::Error> {
                let repr = <$repr>::from_ne_bytes(wire.to_ne_bytes());
                Ok(repr.try_into()?)
            }
        }
    };
    (Fixed<$t:ty>, $w:path) => {
        impl Encoding for Fixed<$t> {
            type Wire = $w;

            type Repr = $t;
            type Error = Infallible;

            fn decode(
                wire: <Self::Wire as NumericWireType>::Repr,
            ) -> Result<Self::Repr, Infallible> {
                Ok(<$t>::from_ne_bytes(wire.to_ne_bytes()))
            }
        }
    };
}

impl_encoding!(Varint<i32>, i64);
impl_encoding!(Varint<i64>, i64);
impl_encoding!(Varint<u32>, u64);
impl_encoding!(Varint<u64>, u64);
impl_encoding!(Varint<ZigZag<i32>>, u32);
impl_encoding!(Varint<ZigZag<i64>>, u64);
impl_encoding!(Fixed<u64>, super::I64);
impl_encoding!(Fixed<u32>, super::I32);
impl_encoding!(Fixed<i64>, super::I64);
impl_encoding!(Fixed<i32>, super::I32);
impl_encoding!(Fixed<f64>, super::I64);
impl_encoding!(Fixed<f32>, super::I32);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sint32_decode() {
        let original = [
            0,
            -1,
            1,
            -2,
            i16::MIN.into(),
            i16::MAX.into(),
            i32::MIN,
            i32::MAX,
        ];
        let original_and_encoded = original.map(|v| {
            (v, {
                let r: u32 = (v >> 31) as u32;
                let l: u32 = v as u32;
                (l << 1) ^ r
            })
        });

        for (original, encoded) in original_and_encoded {
            let decoded = zigzag_decode(encoded) as i32;
            assert_eq!(decoded, original, "encoded = {encoded}");
        }
    }

    #[test]
    fn sint64_decode() {
        let original = [
            0,
            -1,
            1,
            -2,
            i32::MIN.into(),
            i32::MAX.into(),
            i64::MIN,
            i64::MAX,
        ];
        let original_and_encoded = original.map(|v| {
            (v, {
                let r = (v >> 63) as u64;
                let l = v as u64;
                (l << 1) ^ r
            })
        });

        for (original, encoded) in original_and_encoded {
            let decoded = zigzag_decode(encoded) as i64;
            assert_eq!(decoded, original, "encoded = {encoded}");
        }
    }
}
