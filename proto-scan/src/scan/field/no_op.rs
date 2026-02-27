use std::convert::Infallible;

use crate::scan::field::OnScanField;
use crate::scan::{
    GroupOp, IntoResettable, IntoScanner, NumericField, Resettable, ScanTypes, StopScan,
};
use crate::wire::LengthDelimited;

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Default)]
pub struct NoOp;

impl ScanTypes for NoOp {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl OnScanField for NoOp {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Ok(None)
    }
}

impl Resettable for NoOp {
    fn reset(&mut self) {}
}

impl IntoResettable for NoOp {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl IntoScanner for NoOp {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}
