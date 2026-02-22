use crate::proto;

mod custom_field_scanner;
mod embedded_message;
mod read_all;
mod repeated_scalar;
mod single_scalar;

pub(super) fn example_msg() -> crate::prost_proto::ScanExample {
    use crate::prost_proto::*;
    ScanExample {
        repeated_msg: vec![
            MultiFieldMessage {
                id: 1,
                name: "ABC".to_string(),
                flag: true,
            },
            MultiFieldMessage {
                id: 2,
                name: "DEF".to_string(),
                flag: false,
            },
        ],
        single_msg: Some(MultiFieldMessage {
            name: "a".to_owned(),
            id: 2,
            flag: true,
        }),
        repeated_bool: vec![true, true, false, false],
        single_bool: Some(true),
        oneof_group: Some(scan_example::OneofGroup::OneofFixed32(11111111)),
        single_fixed64: Some(123456789),
    }
}

enum InputKind {
    Empty,
    Full,
}

impl InputKind {
    fn into_example_msg(self) -> crate::prost_proto::ScanExample {
        match self {
            InputKind::Empty => Default::default(),
            InputKind::Full => example_msg(),
        }
    }

    fn into_single_field_types(self) -> crate::prost_proto::SingleFieldTypes {
        crate::prost_proto::SingleFieldTypes {
            int32_field: -111111,
            int64_field: -11823844323454654,
            uint32_field: 874839458,
            uint64_field: 23478204893922,
            sint32_field: -371840583,
            sint64_field: -173748299301934928,
            bool_field: true,
            enum_field: crate::prost_proto::EnumType::One.into(),
            fixed64_field: 73294810928097023,
            sfixed64_field: -13649537238187435,
            double_field: 0.123456,
            fixed32_field: 372943813,
            sfixed32_field: -17348172,
            float_field: -0.08776,
        }
    }
}
