#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::Into)]
pub struct FieldNumber(pub(crate) u32);

#[derive(Copy, Clone, Debug)]
pub struct InvalidFieldNumber(pub u32);

impl TryFrom<u32> for FieldNumber {
    type Error = InvalidFieldNumber;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value < 1 << 29 {
            return Ok(Self(value));
        }
        Err(InvalidFieldNumber(value))
    }
}

impl PartialEq<u32> for FieldNumber {
    fn eq(&self, other: &u32) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<FieldNumber> for u32 {
    fn eq(&self, other: &FieldNumber) -> bool {
        self.eq(&other.0)
    }
}
