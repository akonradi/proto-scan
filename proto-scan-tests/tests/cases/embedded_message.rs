use prost::Message as _;
use proto_scan::scan::field::{Save, ScanOptionalMessage, Write};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();
    let mut saved_id = None;

    let scanner = proto::ScanExample::scanner().single_msg(
        proto::MultiFieldMessage::scanner()
            .flag(Save)
            .id(Write(&mut saved_id))
            .empty_if_not_present(),
    );
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
            flag: input.single_msg.unwrap_or_default().flag
        }
    );
}

#[test]
fn scan_concatenated() {
    // If the input is the concatenation of two messages, where both have an
    // embedded message but only the first sets a field in that embedded
    // message, the first's set field should still be saved.
    let mut input = InputKind::Full.into_example_msg();
    let first_id = input.single_msg.as_ref().unwrap().id;
    assert_ne!(first_id, 0);

    let mut bytes = input.encode_to_vec();
    input.single_msg = Some(Default::default());
    bytes.extend(input.encode_to_vec());

    let mut saved_id = 55555555;
    let mut saved_name = "a name".to_owned();
    let scanner = proto::ScanExample::scanner().single_msg(
        proto::MultiFieldMessage::scanner()
            .id(Write(&mut saved_id))
            .name(Write(&mut saved_name))
            .empty_if_not_present(),
    );
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

    assert_eq!(saved_id, first_id);
}

impl InputKind {
    fn into_optional_msg(self) -> crate::prost_proto::WithOptionals {
        use crate::prost_proto::{OptionalBoolWrapper, WithOptionals, with_optionals::Oneof};

        match self {
            Self::Empty => Default::default(),
            Self::Full => WithOptionals {
                optional_bool: Some(false),
                bool_wrapper: Some(OptionalBoolWrapper { value: Some(false) }),
                oneof: Some(Oneof::MessageField(OptionalBoolWrapper {
                    value: Some(false),
                })),
                other_bool_wrapper: Some(OptionalBoolWrapper { value: Some(false) }),
            },
        }
    }
}

#[test_case(Empty)]
#[test_case(Full)]
fn optional_embedded(input: InputKind) {
    let input = input.into_optional_msg();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithOptionals::scanner()
        .optional_bool(Save)
        .bool_wrapper(proto::OptionalBoolWrapper::scanner().value(Save))
        .oneof(
            proto::with_optionals::Oneof::scanner()
                .bool_field(Save)
                .message_field(proto::OptionalBoolWrapper::scanner().value(Save)),
        )
        .other_bool_wrapper(
            proto::OptionalBoolWrapper::scanner()
                .value(Save)
                .empty_if_not_present(),
        );
    let proto::ScanWithOptionalsOutput {
        optional_bool,
        bool_wrapper,
        oneof,
        other_bool_wrapper,
    } = scanner.scan(bytes.as_slice()).read_all().unwrap();

    assert_eq!(optional_bool, input.optional_bool);
    assert_eq!(
        bool_wrapper.map(|b| b.value),
        input.bool_wrapper.map(|b| b.value)
    );
    assert_eq!(
        other_bool_wrapper.value,
        input.other_bool_wrapper.and_then(|b| b.value)
    );
    assert_eq!(
        oneof,
        input.oneof.map(|o| {
            use proto_scan_tests::prost_proto::{OptionalBoolWrapper, with_optionals::Oneof};
            match o {
                Oneof::BoolField(b) => proto::with_optionals::ScanOneofOutput::BoolField(b),
                Oneof::MessageField(OptionalBoolWrapper { value }) => {
                    proto::with_optionals::ScanOneofOutput::MessageField(
                        proto::ScanOptionalBoolWrapperOutput { value },
                    )
                }
            }
        })
    );
}
