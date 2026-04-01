use prost::Message as _;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn enum_is_int(input: InputKind) {
    let input = match input {
        InputKind::Empty => Default::default(),
        InputKind::Full => crate::prost_proto::WithRepeats {
            packed_enum: vec![1, 2, 3],
            ..Default::default()
        },
    };
    let bytes = input.encode_to_vec();

    let scanner = proto::WithRepeats::scanner().packed_enum(Save);
    let result = scanner.scan(bytes.as_slice()).read_all().unwrap();
    assert_eq!(result.packed_enum, input.packed_enum);
}
