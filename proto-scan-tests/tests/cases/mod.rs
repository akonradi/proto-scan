use crate::proto;

mod read_all;
mod single_scalar;
mod repeated_scalar;

pub(super) fn example_msg() -> crate::prost_proto::ScanExample {
    use crate::prost_proto::*;
    ScanExample {
        repeated_msg: vec![MultiFieldMessage {
            id: 1,
            name: "ABC".to_string(),
        }],
        single_msg: Some(MultiFieldMessage {
            name: "a".to_owned(),
            id: 2,
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
}
