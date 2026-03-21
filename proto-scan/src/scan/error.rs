use core::convert::Infallible;

use crate::DecodeError;
use crate::read::ReadBytesError;
use crate::scan::encoding::VarintOutOfBounds;
use crate::wire::WrongWireType;

/// Error produced during a protobuf message scan.
#[derive(Clone, Debug, PartialEq, derive_more::From)]
pub enum ScanError<R> {
    /// A protobuf wire format error was encountered.
    #[from(DecodeError<R>, ReadBytesError<R>)]
    Decode(DecodeError<R>),
    /// A varint value exceeded the bounds of the message field's declared type.
    VarintOutOfBounds,
    /// A field had a different wire type than expected.
    WrongWireType,
    /// A string field contained invalid UTF-8 data.
    Utf8,
    /// Protobuf groups were nested deeper than allowed by the scanner.
    ///
    /// For increasing the limit, see [`Scan::with_group_stack`](super::Scan::with_group_stack).
    GroupOverflow,
    /// An EGROUP tag was missing or didn't have a matching SGROUP tag.
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
