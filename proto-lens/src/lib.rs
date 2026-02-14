pub mod read;
pub mod visitor;
pub mod wire;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError<R> {
    Read(R),
    UnexpectedEnd,
    UnterminatedVarint,
    InvalidWireType(u8),
    TooLargeLengthDelimited(u64),
}

impl<R> DecodeError<R> {
    fn map_read<S>(self, f: impl FnOnce(R) -> S) -> DecodeError<S> {
        match self {
            DecodeError::Read(r) => DecodeError::Read(f(r)),
            DecodeError::UnexpectedEnd => DecodeError::UnexpectedEnd,
            DecodeError::UnterminatedVarint => DecodeError::UnterminatedVarint,
            DecodeError::InvalidWireType(w) => DecodeError::InvalidWireType(w),
            DecodeError::TooLargeLengthDelimited(l) => DecodeError::TooLargeLengthDelimited(l),
        }
    }
}
