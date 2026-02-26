use prost::Message as _;
use proto_scan::scan::{ScanMessage as _, Scanner as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn read_all(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();
    let scanner = proto::ScanExample::scanner()
        .save_single_bool()
        .save_single_fixed64()
        .save_repeated_bool();
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanScanExampleOutput {
        single_bool,
        repeated_msg: (),
        single_msg: (),
        repeated_bool,
        oneof_group: (),
        single_fixed64,
    } = read_all.unwrap();

    assert_eq!(single_bool, input.single_bool);
    assert_eq!(single_fixed64, input.single_fixed64);
    assert_eq!(repeated_bool, input.repeated_bool);
}
