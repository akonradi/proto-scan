use prost::Message;
use proto_scan::scan::field::{Fold, SaveCloned};
use proto_scan::scan::field::{Save, Write};
use proto_scan::scan::field::{ScanRepeated as _, WriteCloned};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn save_repeated_bool(input: InputKind) {
    let input = input.into_example_msg();
    let mut save_to = vec![true, true];

    let scanner = proto::ScanExample::scanner().repeated_bool(Write(&mut save_to));
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            let () = Result::unwrap(event);
        }
    }
    assert_eq!(
        save_to,
        [true, true]
            .into_iter()
            .chain(input.repeated_bool)
            .collect::<Vec<_>>()
    );
}

#[test]
fn save_cloned_repeated_message() {
    let input = Full.into_example_msg();
    let scanner = proto::ScanExample::scanner().repeated_msg(
        proto::MultiFieldMessage::scanner()
            .name(Save)
            .repeat_by(SaveCloned),
    );

    let expected = input
        .repeated_msg
        .iter()
        .map(|m| m.name.as_str())
        .collect::<Vec<_>>();

    let input = input.encode_to_vec();
    let proto::ScanScanExampleOutput {
        repeated_msg,
        single_msg: (),
        repeated_bool: (),
        single_bool: (),
        single_fixed64: (),
        oneof_group: (),
    } = scanner.scan(input.as_slice()).read_all().unwrap();

    let saved_names = repeated_msg
        .into_iter()
        .map(
            |proto::ScanMultiFieldMessageOutput {
                 name,
                 id: _,
                 flag: _,
             }| name,
        )
        .collect::<Vec<_>>();

    assert_eq!(saved_names, expected);
}

#[test]
fn fold_repeated_message() {
    let input = Full.into_example_msg();
    let scanner = proto::ScanExample::scanner().repeated_msg(
        proto::MultiFieldMessage::scanner()
            .id(Save)
            .repeat_by(Fold::new(
                |prev, input: proto::ScanMultiFieldMessageOutput<_, i32, _>| prev.id += input.id,
            )),
    );

    let expected: i32 = input.repeated_msg.iter().map(|m| m.id).sum();

    let input = input.encode_to_vec();
    let proto::ScanScanExampleOutput {
        repeated_msg,
        single_msg: (),
        repeated_bool: (),
        single_bool: (),
        single_fixed64: (),
        oneof_group: (),
    } = scanner.scan(input.as_slice()).read_all().unwrap();

    assert_eq!(repeated_msg.unwrap().id, expected);
}

#[test]
fn write_repeated_message() {
    let input = Full.into_example_msg();
    let mut saved = Vec::new();
    let scanner = proto::ScanExample::scanner().repeated_msg(
        proto::MultiFieldMessage::scanner()
            .id(Save)
            .name(Save)
            .repeat_by(WriteCloned(&mut saved)),
    );

    let expected = input
        .repeated_msg
        .iter()
        .map(|m| proto::ScanMultiFieldMessageOutput {
            id: m.id,
            name: m.name.as_str(),
            flag: (),
        })
        .collect::<Vec<_>>();

    let input = input.encode_to_vec();
    let proto::ScanScanExampleOutput {
        repeated_msg: (),
        single_msg: (),
        repeated_bool: (),
        single_bool: (),
        single_fixed64: (),
        oneof_group: (),
    } = scanner.scan(input.as_slice()).read_all().unwrap();

    assert_eq!(saved, expected);
}
