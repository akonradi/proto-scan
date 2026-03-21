use std::cell::RefCell;
use std::collections::HashSet;

use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::OnScanField;
use proto_scan::scan::{IntoScanOutput, IntoScanner, ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[derive(Copy, Clone, Debug)]
struct EmitEvent<F>(F);

impl<M, F> IntoScanner<M> for EmitEvent<F> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl<F> IntoScanOutput for EmitEvent<F> {
    type ScanOutput = F;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<R: ReadTypes, F: FnMut()> OnScanField<R> for EmitEvent<F> {
    fn on_numeric(
        &mut self,
        _value: proto_scan::wire::NumericField,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        self.0();
        Ok(())
    }
    fn on_group(
        &mut self,
        _group: impl proto_scan::scan::GroupDelimited,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        self.0();
        Ok(())
    }
    fn on_length_delimited(
        &mut self,
        _delimited: impl proto_scan::wire::LengthDelimited<ReadTypes = R>,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        self.0();
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
enum Event {
    SingleBool,
    SingleFixed64,
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();

    let event_sink = RefCell::new(HashSet::new());
    let push_event = |event| {
        let event_sink = &event_sink;
        move || {
            event_sink.borrow_mut().insert(event);
        }
    };

    let scanner = proto::ScanExample::scanner()
        .single_bool(EmitEvent(push_event(Event::SingleBool)))
        .single_fixed64(EmitEvent(push_event(Event::SingleFixed64)));

    let mut expected_events = HashSet::new();
    if input.single_bool.is_some_and(|b| b) {
        expected_events.insert(Event::SingleBool);
    }
    if input.single_fixed64.is_some_and(|f| f != 0) {
        expected_events.insert(Event::SingleFixed64);
    }

    let () = scanner
        .scan(bytes.as_slice())
        .into_iter()
        .collect::<Result<(), _>>()
        .unwrap();

    assert_eq!(event_sink.into_inner(), expected_events);
}
