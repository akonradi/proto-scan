use prost::Message;
use proto_scan::scan::{ScanMessage as _, Scanner as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn save_field(input: InputKind) {
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
        .save_int32_field(&mut save_to.int32_field)
        .save_int64_field(&mut save_to.int64_field)
        .save_uint32_field(&mut save_to.uint32_field)
        .save_uint64_field(&mut save_to.uint64_field)
        .save_sint32_field(&mut save_to.sint32_field)
        .save_sint64_field(&mut save_to.sint64_field)
        .save_bool_field(&mut save_to.bool_field)
        .save_fixed64_field(&mut save_to.fixed64_field)
        .save_sfixed64_field(&mut save_to.sfixed64_field)
        .save_double_field(&mut save_to.double_field)
        .save_fixed32_field(&mut save_to.fixed32_field)
        .save_sfixed32_field(&mut save_to.sfixed32_field)
        .save_float_field(&mut save_to.float_field);
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                None => {}
            }
        }
    }

    let expected = proto::ScanSingleFieldTypesOutput {
        int32_field: Some(input.int32_field),
        int64_field: Some(input.int64_field),
        uint32_field: Some(input.uint32_field),
        uint64_field: Some(input.uint64_field),
        sint32_field: Some(input.sint32_field),
        sint64_field: Some(input.sint64_field),
        bool_field: Some(input.bool_field),
        enum_field: (),
        fixed64_field: Some(input.fixed64_field),
        sfixed64_field: Some(input.sfixed64_field),
        double_field: Some(input.double_field),
        fixed32_field: Some(input.fixed32_field),
        sfixed32_field: Some(input.sfixed32_field),
        float_field: Some(input.float_field),
    };
    assert_eq!(save_to, expected);
}

#[test_case(Empty)]
#[test_case(Full)]
fn emit_field(input: InputKind) {
    let input = input.into_single_field_types();

    let scanner = proto::SingleFieldTypes::scanner()
        .emit_int32_field()
        .emit_int64_field()
        .emit_uint32_field()
        .emit_uint64_field()
        .emit_sint32_field()
        .emit_sint64_field()
        .emit_bool_field()
        .emit_fixed64_field()
        .emit_sfixed64_field()
        .emit_double_field()
        .emit_fixed32_field()
        .emit_sfixed32_field()
        .emit_float_field();

    let output = {
        let bytes = input.encode_to_vec();
        scanner.scan(bytes.as_slice()).read_all().unwrap()
    };

    let expected = proto::ScanSingleFieldTypesOutput {
        int32_field: Some(input.int32_field),
        int64_field: Some(input.int64_field),
        uint32_field: Some(input.uint32_field),
        uint64_field: Some(input.uint64_field),
        sint32_field: Some(input.sint32_field),
        sint64_field: Some(input.sint64_field),
        bool_field: Some(input.bool_field),
        enum_field: (),
        fixed64_field: Some(input.fixed64_field),
        sfixed64_field: Some(input.sfixed64_field),
        double_field: Some(input.double_field),
        fixed32_field: Some(input.fixed32_field),
        sfixed32_field: Some(input.sfixed32_field),
        float_field: Some(input.float_field),
    };
    assert_eq!(output, expected);
}
