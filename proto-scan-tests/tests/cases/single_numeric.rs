use prost::Message;
use proto_scan::scan::field::{Save, Write};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn write_field(input: InputKind) {
    let input = input.into_single_field_types();
    let mut save_to = proto::ScanSingleFieldTypesOutput {
        int32_field: None,
        int64_field: None,
        uint32_field: None,
        uint64_field: None,
        sint32_field: None,
        sint64_field: None,
        bool_field: None,
        enum_field: (),
        fixed64_field: None,
        sfixed64_field: None,
        double_field: None,
        fixed32_field: None,
        sfixed32_field: None,
        float_field: None,
    };

    let scanner = proto::SingleFieldTypes::scanner()
        .int32_field(Write(&mut save_to.int32_field))
        .int64_field(Write(&mut save_to.int64_field))
        .uint32_field(Write(&mut save_to.uint32_field))
        .uint64_field(Write(&mut save_to.uint64_field))
        .sint32_field(Write(&mut save_to.sint32_field))
        .sint64_field(Write(&mut save_to.sint64_field))
        .bool_field(Write(&mut save_to.bool_field))
        .fixed64_field(Write(&mut save_to.fixed64_field))
        .sfixed64_field(Write(&mut save_to.sfixed64_field))
        .double_field(Write(&mut save_to.double_field))
        .fixed32_field(Write(&mut save_to.fixed32_field))
        .sfixed32_field(Write(&mut save_to.sfixed32_field))
        .float_field(Write(&mut save_to.float_field));
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                None => {}
            }
        }
    }

    let expected = proto::ScanSingleFieldTypesOutput {
        int32_field: none_if_default(input.int32_field),
        int64_field: none_if_default(input.int64_field),
        uint32_field: none_if_default(input.uint32_field),
        uint64_field: none_if_default(input.uint64_field),
        sint32_field: none_if_default(input.sint32_field),
        sint64_field: none_if_default(input.sint64_field),
        bool_field: none_if_default(input.bool_field),
        enum_field: (),
        fixed64_field: none_if_default(input.fixed64_field),
        sfixed64_field: none_if_default(input.sfixed64_field),
        double_field: none_if_default(input.double_field),
        fixed32_field: none_if_default(input.fixed32_field),
        sfixed32_field: none_if_default(input.sfixed32_field),
        float_field: none_if_default(input.float_field),
    };
    assert_eq!(save_to, expected);
}

#[test_case(Empty)]
#[test_case(Full)]
fn save_field(input: InputKind) {
    let input = input.into_single_field_types();

    let scanner = proto::SingleFieldTypes::scanner()
        .int32_field(Save)
        .int64_field(Save)
        .uint32_field(Save)
        .uint64_field(Save)
        .sint32_field(Save)
        .sint64_field(Save)
        .bool_field(Save)
        .fixed64_field(Save)
        .sfixed64_field(Save)
        .double_field(Save)
        .fixed32_field(Save)
        .sfixed32_field(Save)
        .float_field(Save);

    let output = {
        let bytes = input.encode_to_vec();
        scanner.scan(bytes.as_slice()).read_all().unwrap()
    };

    let expected = proto::ScanSingleFieldTypesOutput {
        int32_field: none_if_default(input.int32_field),
        int64_field: none_if_default(input.int64_field),
        uint32_field: none_if_default(input.uint32_field),
        uint64_field: none_if_default(input.uint64_field),
        sint32_field: none_if_default(input.sint32_field),
        sint64_field: none_if_default(input.sint64_field),
        bool_field: none_if_default(input.bool_field),
        enum_field: (),
        fixed64_field: none_if_default(input.fixed64_field),
        sfixed64_field: none_if_default(input.sfixed64_field),
        double_field: none_if_default(input.double_field),
        fixed32_field: none_if_default(input.fixed32_field),
        sfixed32_field: none_if_default(input.sfixed32_field),
        float_field: none_if_default(input.float_field),
    };
    assert_eq!(output, expected);
}

fn none_if_default<T: Default + PartialEq>(val: T) -> Option<T> {
    (val != T::default()).then_some(val)
}
