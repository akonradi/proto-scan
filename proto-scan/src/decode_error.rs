use crate::read::ReadBytesError;

/// Protobuf wire format decode error.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError<R> {
    /// Underlying read source error.
    Read(R),
    /// An invalid varint was found.
    InvalidVarint,
    /// The stream ended prematurely.
    UnexpectedEnd,
    /// An unknown wire type was encountered.
    InvalidWireType(u8),
    /// A length-delimited field's purported size was too large.
    TooLargeLengthDelimited(u64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::From)]
pub enum DecodeVarintError<R> {
    /// Underlying read source error.
    #[from]
    Read(R),
    /// An invalid varint was found.
    InvalidVarint,
    /// The stream ended prematurely.
    UnexpectedEnd,
}

impl<R> From<DecodeVarintError<R>> for DecodeError<R> {
    fn from(value: DecodeVarintError<R>) -> Self {
        match value {
            DecodeVarintError::Read(r) => Self::Read(r),
            DecodeVarintError::InvalidVarint => Self::InvalidVarint,
            DecodeVarintError::UnexpectedEnd => Self::UnexpectedEnd,
        }
    }
}

impl<R> From<ReadBytesError<R>> for DecodeError<R> {
    fn from(value: ReadBytesError<R>) -> Self {
        match value {
            ReadBytesError::Read(r) => Self::Read(r),
            ReadBytesError::UnexpectedEnd => Self::UnexpectedEnd,
        }
    }
}
