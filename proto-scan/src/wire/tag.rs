use crate::DecodeError;
use crate::read::Read;
use crate::wire::FieldNumber;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Tag {
    pub(crate) wire_type: WireType,
    pub(crate) field_number: FieldNumber,
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
        let (field_number, wire_type) = (value >> 3, (value & 0b111) as u8);
        let wire_type = wire_type
            .try_into()
            .map_err(|_| DecodeError::<R::Error>::InvalidWireType(wire_type))?;

        Ok(Self {
            field_number: FieldNumber(field_number),
            wire_type,
        })
    }

    #[cfg(test)]
    pub(crate) fn serialized(&self) -> Box<[u8]> {
        let Self {
            wire_type,
            field_number,
        } = *self;
        let value = (u32::from(field_number) << 3) | u32::from(wire_type as u8);
        super::serialize_base128_varint(value)
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
                field_number: FieldNumber(1),
                wire_type: WireType::Varint
            })
        )
    }

    #[test]
    fn read_i32() {
        let bytes = [61u8];

        let tag = Tag::read_from(&mut bytes.as_slice());

        assert_eq!(
            tag,
            Ok(Tag {
                field_number: FieldNumber(7),
                wire_type: WireType::I32
            })
        );
    }
}
