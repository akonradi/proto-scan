#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScalarField {
    Varint(u64),
    I64(u64),
    I32(u32),
}

impl ScalarField {
    #[cfg(test)]
    pub(crate) fn serialize(&self) -> Box<[u8]> {
        match self {
            ScalarField::Varint(v) => super::serialize_base128_varint(*v),
            ScalarField::I64(v) => v.to_le_bytes().into(),
            ScalarField::I32(v) => v.to_le_bytes().into(),
        }
    }
}
