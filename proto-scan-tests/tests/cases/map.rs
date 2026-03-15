use prost::Message as _;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_map_message();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithMap::scanner()
        .fixed64_to_i32(Save)
        .fixed64_to_message(Save::with_value(proto::MapValue::scanner().id(Save)))
        .string_to_i32(Save)
        .string_to_message(Save::with_value(proto::MapValue::scanner().id(Save)));

    let output = scanner.scan(&mut bytes.as_slice()).read_all().unwrap();

    let expected = proto::ScanWithMapOutput::<_, _, _, _> {
        fixed64_to_i32: input.fixed64_to_i32,
        fixed64_to_message: input
            .fixed64_to_message
            .into_iter()
            .map(|(k, v)| (k, proto::ScanMapValueOutput { id: v.id }))
            .collect(),
        string_to_i32: input
            .string_to_i32
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect(),
        string_to_message: input
            .string_to_message
            .iter()
            .map(|(k, v)| (k.as_str(), proto::ScanMapValueOutput { id: v.id }))
            .collect(),
    };
    assert_eq!(output, expected)
}
