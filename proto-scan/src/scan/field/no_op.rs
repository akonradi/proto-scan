use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupOp, IntoResettable, IntoScanOutput, IntoScanner, NumericField, Resettable, ScanCallbacks,
    StopScan,
};
use crate::wire::LengthDelimited;

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Default)]
pub struct NoOp;

impl<R: ReadTypes> OnScanField<R> for NoOp {
    type ScanEvent = Infallible;

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

impl<R: ReadTypes, F> ScanCallbacks<R, F> for NoOp {
    type ScanEvent = ();

    fn on_numeric(&mut self, _field: F, _value: NumericField) -> Result<Self::ScanEvent, StopScan> {
        Ok(())
    }

    fn on_group(&mut self, _field: F, _op: GroupOp) -> Result<Self::ScanEvent, StopScan> {
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _field: F,
        _delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, StopScan> {
        Ok(())
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

impl<T: ?Sized> IntoScanner<T> for NoOp {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl IntoScanOutput for NoOp {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}
