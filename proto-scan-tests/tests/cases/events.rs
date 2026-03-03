use std::collections::HashSet;

use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::OnScanField;
use proto_scan::scan::{IntoScanOutput, IntoScanner, ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[derive(Copy, Clone, Debug)]
struct EmitEvent<T>(T);

impl<T> IntoScanner for EmitEvent<T> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl<T> IntoScanOutput for EmitEvent<T> {
    type ScanOutput = T;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<R: ReadTypes, T: Clone> OnScanField<R> for EmitEvent<T> {
    type ScanEvent = T;
    fn on_numeric(
        &mut self,
        _value: proto_scan::wire::NumericField,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Ok(Some(self.0.clone()))
    }
    fn on_group(
        &mut self,
        _op: proto_scan::wire::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Ok(Some(self.0.clone()))
    }
    fn on_length_delimited(
        &mut self,
        _delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Ok(Some(self.0.clone()))
    }
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();

    let scanner = proto::ScanExample::scanner()
        .single_bool(EmitEvent("bool"))
        .single_fixed64(EmitEvent(b"fixed64"));

    let mut expected_events = HashSet::new();
    if input.single_bool.is_some_and(|b| b) {
        expected_events.insert(Some(proto::ScanScanExampleEvent::SingleBool("bool")));
    }
    if input.single_fixed64.is_some_and(|f| f != 0) {
        expected_events.insert(Some(proto::ScanScanExampleEvent::SingleFixed64(b"fixed64")));
    }
    if !bytes.is_empty() {
        expected_events.insert(None);
    }

    let events = scanner
        .scan(bytes.as_slice())
        .into_iter()
        .inspect(|e| println!("{e:?}"))
        .collect::<Result<HashSet<_>, _>>()
        .unwrap();

    assert_eq!(events, expected_events);
}
