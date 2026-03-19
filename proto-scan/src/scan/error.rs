use core::convert::Infallible;

use crate::DecodeError;
use crate::scan::encoding::VarintOutOfBounds;
use crate::wire::WrongWireType;

#[derive(Clone, Debug, PartialEq, derive_more::From)]
pub enum ScanError<R> {
    #[from]
    Decode(DecodeError<R>),
    VarintOutOfBounds,
    WrongWireType,
    Utf8,
    GroupOverflow,
    GroupMismatch,
}

impl<R> From<Infallible> for ScanError<R> {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl<R> From<WrongWireType> for ScanError<R> {
    fn from(WrongWireType: WrongWireType) -> Self {
        Self::WrongWireType
    }
}

impl<R> From<VarintOutOfBounds> for ScanError<R> {
    fn from(_: VarintOutOfBounds) -> Self {
        Self::VarintOutOfBounds
    }
}

impl<R> From<core::str::Utf8Error> for ScanError<R> {
    fn from(_value: core::str::Utf8Error) -> Self {
        Self::Utf8
    }
}
