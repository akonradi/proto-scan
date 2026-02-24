use std::convert::Infallible;

use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, Resettable, ScalarField, ScanTypes, StopScan};
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

    fn on_scalar(&mut self, _value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan> {
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
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {}
    fn reset(&mut self, (): Self::Mark) {}
}
