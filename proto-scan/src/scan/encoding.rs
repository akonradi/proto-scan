use std::convert::Infallible;
use std::marker::PhantomData;

use crate::scan::StopScan;
use crate::wire::ScalarWireType;

pub struct Varint<T>(PhantomData<T>);
pub struct Fixed<T>(PhantomData<T>);
pub struct ZigZag<T>(PhantomData<T>);

pub trait Encoding {
    type Wire: ScalarWireType;
    type Repr: Copy;
    type Error: Into<super::StopScan>;

    fn decode(wire: <Self::Wire as ScalarWireType>::Repr) -> Result<Self::Repr, Self::Error>;
}

impl Encoding for Varint<bool> {
    type Wire = super::Varint;

    type Repr = bool;
    type Error = super::StopScan;

    fn decode(wire: <Self::Wire as ScalarWireType>::Repr) -> Result<bool, super::StopScan> {
        match wire {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(super::StopScan),
        }
    }
}

impl Encoding for Varint<i32> {
    type Wire = super::Varint;

    type Repr = i32;
    type Error = super::StopScan;

    fn decode(wire: <Self::Wire as ScalarWireType>::Repr) -> Result<i32, super::StopScan> {
        let v : u32 = wire.try_into().ok().ok_or(StopScan)?;
        Ok(v as i32)
    }
}


impl Encoding for Fixed<u64> {
    type Wire = super::I64;

    type Repr = u64;
    type Error = Infallible;

    fn decode(wire: u64) -> Result<u64, Infallible> {
        Ok(wire)
    }
}
