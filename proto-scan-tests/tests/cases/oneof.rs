use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::Save;
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, ScanCallbacks, ScanMessage as _, ScannerBuilder as _,
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
            .oneof_bool(Save)
            .oneof_fixed_32(Save)
            .oneof_message(proto::MultiFieldMessage::scanner().id(Save)),
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
                proto::scan_example::ScanOneofGroupOutput::OneofBool(b),
            crate::prost_proto::scan_example::OneofGroup::OneofFixed32(f) =>
                proto::scan_example::ScanOneofGroupOutput::OneofFixed32(f),
            crate::prost_proto::scan_example::OneofGroup::OneofMessage(_) =>
                proto::scan_example::ScanOneofGroupOutput::OneofMessage(
                    proto::ScanMultiFieldMessageOutput {
                        id: 0,
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
            .oneof_message(proto::MultiFieldMessage::scanner().name(Save)),
    );
    let scan = scanner.scan(bytes.as_slice());

    let oneof_group = scan.read_all().unwrap().oneof_group;
    let found = oneof_group.map(|g| match g {
        proto::scan_example::ScanOneofGroupOutput::OneofMessage(m) => m.name,
        _ => "",
    });
    assert_eq!(found, Some("abc123"));
}

#[test]
fn merge_embedded_message() {
    let input = crate::prost_proto::WithOneofRepeat {
        oneof_group: Some(
            crate::prost_proto::with_oneof_repeat::OneofGroup::OneofMessage(
                crate::prost_proto::WithRepeats {
                    packed_bool: vec![true, false],
                    ..Default::default()
                },
            ),
        ),
    };
    let bytes = input.encode_to_vec().repeat(2);

    let scanner = proto::WithOneofRepeat::scanner().oneof_group(
        proto::with_oneof_repeat::OneofGroup::scanner()
            .oneof_message(proto::WithRepeats::scanner().packed_bool(Save)),
    );

    let output = scanner.scan(bytes.as_slice()).read_all().unwrap();

    assert_eq!(
        output,
        proto::ScanWithOneofRepeatOutput {
            oneof_group: Some(
                proto::with_oneof_repeat::ScanOneofGroupOutput::OneofMessage(
                    proto::ScanWithRepeatsOutput {
                        packed_bool: vec![true, false].repeat(2),
                        ..Default::default()
                    }
                )
            )
        }
    );
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
                proto::scan_example::ScanOneofGroupFieldNum::OneofBool,
            crate::prost_proto::scan_example::OneofGroup::OneofFixed32(_) =>
                proto::scan_example::ScanOneofGroupFieldNum::OneofFixed32,
            crate::prost_proto::scan_example::OneofGroup::OneofMessage(_) =>
                proto::scan_example::ScanOneofGroupFieldNum::OneofMessage,
        })
    );
}

struct SaveLastFieldNumber<'t, F>(&'t mut Option<F>);
impl<M, F> IntoScanner<M> for SaveLastFieldNumber<'_, F> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl<F> IntoScanOutput for SaveLastFieldNumber<'_, F> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<R: ReadTypes, F> ScanCallbacks<R, F> for SaveLastFieldNumber<'_, F> {
    type ScanEvent = ();
    fn on_numeric(
        &mut self,
        field: F,
        _value: proto_scan::wire::NumericField,
    ) -> Result<Self::ScanEvent, proto_scan::scan::ScanError<R::Error>> {
        *self.0 = Some(field);
        Ok(())
    }
    fn on_length_delimited(
        &mut self,
        field: F,
        _delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, proto_scan::scan::ScanError<R::Error>> {
        *self.0 = Some(field);
        Ok(())
    }
    fn on_group(
        &mut self,
        field: F,
        _group: impl proto_scan::scan::GroupDelimited,
    ) -> Result<Self::ScanEvent, proto_scan::scan::ScanError<R::Error>> {
        *self.0 = Some(field);
        Ok(())
    }
}
