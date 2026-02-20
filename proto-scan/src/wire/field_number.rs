#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::Into)]
pub struct FieldNumber(pub(crate) u32);

#[derive(Copy, Clone, Debug)]
pub struct InvalidFieldNumber(pub u32);

impl FieldNumber {
    pub const fn new(field_number: u32) -> Result<Self, InvalidFieldNumber> {
        if field_number < 1 << 29 {
            return Ok(Self(field_number));
        }
        Err(InvalidFieldNumber(field_number))
    }
}

impl TryFrom<u32> for FieldNumber {
    type Error = InvalidFieldNumber;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
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
