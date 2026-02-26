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

#[test]
fn scan_concatenated() {
    // If the input is the concatenation of two messages, where both have an
    // embedded message but only the first sets a field in that embedded
    // message, seeing the field set in the first message should not be observable
    // in terms of the mut ref passed to the write_ builder method.
    let mut input = InputKind::Full.into_example_msg();
    let mut bytes = input.encode_to_vec();
    input.single_msg = Some(Default::default());
    bytes.extend(input.encode_to_vec());

    let mut saved_id = 55555555;
    let mut saved_name = "a name".to_owned();
    let scanner = proto::ScanExample::scanner().single_msg(Message::new(
        proto::MultiFieldMessage::scanner()
            .write_id(&mut saved_id)
            .write_name(&mut saved_name),
    ));
    let proto::ScanScanExampleOutput {
        repeated_msg: (),
        single_msg:
            proto::ScanMultiFieldMessageOutput {
                flag: (),
                id: (),
                name: (),
            },
        repeated_bool: (),
        single_bool: (),
        single_fixed64: (),
        oneof_group: (),
    } = scanner.scan(bytes.as_slice()).read_all().unwrap();

    assert_eq!(saved_id, 55555555)
}
