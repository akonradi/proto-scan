use std::convert::Infallible;

use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, OnScanOneof, ScanMessage as _, ScannerBuilder as _,
};
use proto_scan::wire::FieldNumber;
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn save_field(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();

    let scanner = proto::ScanExample::scanner().oneof_group(
        proto::scan_example::OneofGroup::scanner()
            .save_OneofBool()
            .save_OneofFixed32()
            .scan_OneofMessage(proto::MultiFieldMessage::scanner().save_id()),
    );
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanScanExampleOutput {
        single_bool: (),
        repeated_msg: (),
        single_msg: (),
        repeated_bool: (),
        oneof_group,
        single_fixed64: (),
    } = read_all.unwrap();

    assert_eq!(
        oneof_group,
        input.oneof_group.map(|g| match g {
            crate::prost_proto::scan_example::OneofGroup::OneofBool(b) =>
                proto::scan_example::ScanOneofGroupOutput::OneofBool(b.then_some(true)),
            crate::prost_proto::scan_example::OneofGroup::OneofFixed32(f) =>
                proto::scan_example::ScanOneofGroupOutput::OneofFixed32(Some(f)),
            crate::prost_proto::scan_example::OneofGroup::OneofMessage(_) =>
                proto::scan_example::ScanOneofGroupOutput::OneofMessage(
                    proto::ScanMultiFieldMessageOutput {
                        id: None,
                        ..Default::default()
                    }
                ),
        })
    );
}

#[test]
fn save_oneof_message_field() {
    let bytes = crate::prost_proto::ScanExample {
        oneof_group: Some(crate::prost_proto::scan_example::OneofGroup::OneofMessage(
            crate::prost_proto::MultiFieldMessage {
                name: "abc123".into(),
                ..Default::default()
            },
        )),
        ..Default::default()
    }
    .encode_to_vec();
    let scanner = proto::ScanExample::scanner().oneof_group(
        proto::scan_example::OneofGroup::scanner()
            .scan_OneofMessage(proto::MultiFieldMessage::scanner().save_name()),
    );
    let scan = scanner.scan(bytes.as_slice());

    let oneof_group = scan.read_all().unwrap().oneof_group;
    let found = oneof_group.and_then(|g| match g {
        proto::scan_example::ScanOneofGroupOutput::OneofMessage(m) => m.name,
        _ => None,
    });
    assert_eq!(found, Some("abc123"));
}

#[test_case(Empty)]
#[test_case(Full)]
fn custom_scanner(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();
    let mut oneof_field = None;

    let scanner = proto::ScanExample::scanner().oneof_group(SaveLastFieldNumber(&mut oneof_field));
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanScanExampleOutput {
        single_bool: (),
        repeated_msg: (),
        single_msg: (),
        repeated_bool: (),
        oneof_group: (),
        single_fixed64: (),
    } = read_all.unwrap();

    let input = dbg!(input);
    assert_eq!(
        oneof_field,
        input.oneof_group.map(|g| FieldNumber::new(match g {
            crate::prost_proto::scan_example::OneofGroup::OneofBool(_) => 5,
            crate::prost_proto::scan_example::OneofGroup::OneofFixed32(_) => 6,
            crate::prost_proto::scan_example::OneofGroup::OneofMessage(_) => 7,
        })
        .unwrap())
    );
}

struct SaveLastFieldNumber<'t>(&'t mut Option<FieldNumber>);
impl IntoScanner for SaveLastFieldNumber<'_> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl IntoScanOutput for SaveLastFieldNumber<'_> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<R: ReadTypes> OnScanOneof<R> for SaveLastFieldNumber<'_> {
    type ScanEvent = Infallible;
    fn on_numeric(
        &mut self,
        field: FieldNumber,
        _value: proto_scan::wire::NumericField,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        _delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
    fn on_group(
        &mut self,
        field: FieldNumber,
        _op: proto_scan::wire::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
}
