use prost::Message;
use proto_scan::scan::field::{Save, Write};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn write_bytes(input: InputKind) {
    let input = input.into_bytes_field_types();
    let mut save_to = proto::ScanBytesFieldTypesOutput {
        bytes_field: Vec::new(),
        string_field: String::new(),
        repeated_bytes_field: (),
        repeated_string_field: (),
    };

    let scanner = proto::BytesFieldTypes::scanner()
        .bytes_field(Write(&mut save_to.bytes_field))
        .string_field(Write(&mut save_to.string_field));
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                None => {}
            }
        }
    }

    let expected = proto::ScanBytesFieldTypesOutput {
        bytes_field: input.bytes_field,
        string_field: input.string_field,
        repeated_bytes_field: (),
        repeated_string_field: (),
    };
    assert_eq!(save_to, expected);
}

#[test_case(Empty)]
#[test_case(Full)]
fn save_bytes(input: InputKind) {
    let input = input.into_bytes_field_types();

    let scanner = proto::BytesFieldTypes::scanner()
        .bytes_field(Save)
        .string_field(Save);
    let bytes = input.encode_to_vec();
    let output = { scanner.scan(bytes.as_slice()).read_all().unwrap() };

    let expected = proto::ScanBytesFieldTypesOutput {
        bytes_field: input.bytes_field.as_slice(),
        string_field: input.string_field.as_ref(),
        repeated_bytes_field: (),
        repeated_string_field: (),
    };
    assert_eq!(output, expected);
}
