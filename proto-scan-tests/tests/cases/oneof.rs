use std::convert::Infallible;

use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, OnScanOneof, ScanMessage as _, ScannerBuilder as _,
};
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
            .save_oneof_bool()
            .save_oneof_fixed_32()
            .scan_oneof_message(proto::MultiFieldMessage::scanner().save_id()),
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
            .scan_oneof_message(proto::MultiFieldMessage::scanner().save_name()),
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
        input.oneof_group.map(|g| match g {
            crate::prost_proto::scan_example::OneofGroup::OneofBool(_) =>
                proto::scan_example::ScanOneofGroupOutput::OneofBool(()),
            crate::prost_proto::scan_example::OneofGroup::OneofFixed32(_) =>
                proto::scan_example::ScanOneofGroupOutput::OneofFixed32(()),
            crate::prost_proto::scan_example::OneofGroup::OneofMessage(_) =>
                proto::scan_example::ScanOneofGroupOutput::OneofMessage(()),
        })
    );
}

struct SaveLastFieldNumber<'t>(&'t mut Option<proto::scan_example::ScanOneofGroupOutput>);
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

impl<R: ReadTypes> OnScanOneof<R, proto::scan_example::ScanOneofGroupOutput>
    for SaveLastFieldNumber<'_>
{
    type ScanEvent = Infallible;
    fn on_numeric(
        &mut self,
        field: proto::scan_example::ScanOneofGroupOutput,
        _value: proto_scan::wire::NumericField,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
    fn on_length_delimited(
        &mut self,
        field: proto::scan_example::ScanOneofGroupOutput,
        _delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
    fn on_group(
        &mut self,
        field: proto::scan_example::ScanOneofGroupOutput,
        _op: proto_scan::wire::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(field);
        Ok(None)
    }
}
