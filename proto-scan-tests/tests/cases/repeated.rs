use std::convert::Infallible;

use prost::Message;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::{OnScanField, Save, Write};
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, ScanCallbacks, ScanMessage as _, ScannerBuilder as _, StopScan,
};
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
            match Result::unwrap(event) {
                None | Some(proto::ScanScanExampleEvent::OneofGroup(())) => (),
            }
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

struct SaveRepeated<S>(S);

struct SaveRepeatedMessageScanner<S: IntoScanOutput>(S, Vec<S::ScanOutput>);

impl<S: IntoScanner<T>, T>
    IntoScanner<proto_scan::scan::Repeated<proto_scan::scan::field::Message<T>>>
    for SaveRepeated<S>
{
    type Scanner<R: proto_scan::read::ReadTypes> = SaveRepeatedMessageScanner<S::Scanner<R>>;
    fn into_scanner<R: proto_scan::read::ReadTypes>(self) -> Self::Scanner<R> {
        SaveRepeatedMessageScanner(self.0.into_scanner(), Vec::new())
    }
}

impl<S: IntoScanOutput> IntoScanOutput for SaveRepeatedMessageScanner<S> {
    type ScanOutput = Vec<S::ScanOutput>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.1
    }
}

impl<R: ReadTypes, S: Clone + ScanCallbacks<R>> OnScanField<R> for SaveRepeatedMessageScanner<S> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: proto_scan::wire::NumericField,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Err(StopScan)
    }

    fn on_group(
        &mut self,
        _op: proto_scan::wire::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        let scanner = self.0.clone();
        let mut scanner = proto_scan::scan::field::Message::new(scanner);
        let event = scanner.on_length_delimited(delimited)?;
        self.1.push(scanner.into_scan_output());
        Ok(event)
    }
}

#[test]
fn custom_repeated_message() {
    let input = Full.into_example_msg();
    let scanner = proto::ScanExample::scanner()
        .repeated_msg(SaveRepeated(proto::MultiFieldMessage::scanner().name(Save)));

    let expected = input
        .repeated_msg
        .iter()
        .map(|m| Some(m.name.as_str()))
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
