#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DecodeError<R> {
    Read(R),
    UnexpectedEnd,
    UnterminatedVarint,
    InvalidWireType(u8),
    TooLargeLengthDelimited(u64),
}