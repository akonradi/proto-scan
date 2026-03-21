use crate::read::ReadBytesError;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError<R> {
    Read(R),
    InvalidVarint,
    UnexpectedEnd,
    InvalidWireType(u8),
    TooLargeLengthDelimited(u64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::From)]
pub enum DecodeVarintError<R> {
    #[from]
    Read(R),
    InvalidVarint,
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
