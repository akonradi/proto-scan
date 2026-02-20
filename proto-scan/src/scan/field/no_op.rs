use std::convert::Infallible;

use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, ScalarField, StopScan};
use crate::wire::LengthDelimited;

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Default)]
pub struct NoOp;

impl OnScanField for NoOp {
    type ScanEvent = Infallible;

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
