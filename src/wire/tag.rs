use crate::DecodeError;
use crate::read::Read;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Tag {
    pub(crate) wire_type: WireType,
    pub(crate) field_number: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, derive_more::TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub(crate) enum WireType {
    Varint,
    I64,
    LengthDelimited,
    Sgroup,
    Egroup,
    I32,
}

impl Tag {
    pub(crate) fn read_from<R: Read>(r: &mut R) -> Result<Self, DecodeError<R::Error>> {
        let value: u32 = super::parse_base128_varint(r)?;
        let (field_number, wire_type) = (value >> 3, (value & 0b11) as u8);
        let wire_type = wire_type
            .try_into()
            .map_err(|_| DecodeError::<R::Error>::InvalidWireType(wire_type))?;

        Ok(Self {
            field_number,
            wire_type,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_tag() {
        let bytes = [0x08, 0x96, 0x01];

        let tag = Tag::read_from(&mut bytes.as_slice());
        assert_eq!(
            tag,
            Ok(Tag {
                field_number: 1,
                wire_type: WireType::Varint
            })
        )
    }
}
