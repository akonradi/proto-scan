use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::encoding::Varint;
use proto_scan::scan::field::OnScanField;
use proto_scan::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, ScanMessage as _, ScannerBuilder as _,
};
use proto_scan::wire::NumericField;
use test_case::test_case;

use super::*;
use InputKind::*;

struct CustomScanner<'t>(&'t mut Option<NumericField>);

#[derive(Debug, Default, PartialEq)]
struct CustomOutput;

impl<R: ReadTypes> OnScanField<R> for CustomScanner<'_> {
    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        *self.0 = Some(value);
        Ok(())
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl proto_scan::wire::LengthDelimited,
    ) -> Result<(), proto_scan::scan::ScanError<R::Error>> {
        Ok(())
    }
}

impl IntoScanner<Option<Varint<bool>>> for CustomScanner<'_> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl IntoScanOutput for CustomScanner<'_> {
    type ScanOutput = CustomOutput;

    fn into_scan_output(self) -> Self::ScanOutput {
        CustomOutput
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
        input.single_bool.map(|b| NumericField::Varint(b as u64))
    );
}
