#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NumericField {
    Varint(u64),
    I64(u64),
    I32(u32),
}

impl NumericField {
    #[cfg(test)]
    pub(crate) fn serialize(&self) -> Box<[u8]> {
        match self {
            NumericField::Varint(v) => super::serialize_base128_varint(*v),
            NumericField::I64(v) => v.to_le_bytes().into(),
            NumericField::I32(v) => v.to_le_bytes().into(),
        }
    }
}
