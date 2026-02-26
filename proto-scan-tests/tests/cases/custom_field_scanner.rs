use prost::Message as _;
use proto_scan::scan::field::OnScanField;
use proto_scan::scan::{ScanMessage as _, ScanTypes, Scanner as _};
use proto_scan::wire::ScalarField;
use test_case::test_case;

use super::*;
use InputKind::*;

struct CustomScanner<'t>(&'t mut Option<ScalarField>);
#[derive(Debug, PartialEq)]
struct CustomEvent;
#[derive(Debug, Default, PartialEq)]
struct CustomOutput;

impl ScanTypes for CustomScanner<'_> {
    type ScanEvent = CustomEvent;
    type ScanOutput = CustomOutput;
}

impl OnScanField for CustomScanner<'_> {
    fn into_output(self) -> Self::ScanOutput {
        CustomOutput
    }

    fn on_scalar(
        &mut self,
        value: ScalarField,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        *self.0 = Some(value);
        Ok(Some(CustomEvent))
    }

    fn on_group(
        &mut self,
        _op: proto_scan::wire::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl proto_scan::wire::LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, proto_scan::scan::StopScan> {
        Ok(None)
    }
}

#[test_case(Empty)]
#[test_case(Full)]
fn custom_scanner(input: InputKind) {
    let input = input.into_example_msg();
    let bytes = input.encode_to_vec();
    let mut out = None;
    let scanner = proto::ScanExample::scanner().single_bool(CustomScanner(&mut out));
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanScanExampleOutput {
        single_bool,
        repeated_msg: (),
        single_msg: (),
        repeated_bool: (),
        oneof_group: (),
        single_fixed64: (),
    } = read_all.unwrap();

    assert_eq!(single_bool, CustomOutput);
    assert_eq!(
        out,
        input.single_bool.map(|b| ScalarField::Varint(b as u64))
    );
}
