use prost::Message as _;
use proto_scan::scan::field::Message;
use proto_scan::scan::{ScanMessage as _, Scanner as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();
    let mut saved_id = None;

    let scanner = proto::ScanExample::scanner().single_msg(Message::new(
        proto::MultiFieldMessage::scanner()
            .save_flag()
            .write_id(&mut saved_id),
    ));
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanScanExampleOutput {
        single_bool: (),
        repeated_msg: (),
        single_msg,
        repeated_bool: (),
        oneof_group: (),
        single_fixed64: (),
    } = read_all.unwrap();

    let input = dbg!(input);
    assert_eq!(
        single_msg,
        proto::ScanMultiFieldMessageOutput {
            name: (),
            id: (),
            flag: input.single_msg.unwrap_or_default().flag.then_some(true)
        }
    );
}
