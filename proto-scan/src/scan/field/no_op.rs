use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupOp, IntoResettableScanner, IntoScanOutput, IntoScanner, NumericField, ResettableScanner,
    ScanCallbacks, ScanError,
};
use crate::wire::LengthDelimited;

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Copy, Clone, Default)]
pub struct NoOp;

impl<R: ReadTypes> OnScanField<R> for NoOp {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Ok(None)
    }
}

impl<R: ReadTypes, F> ScanCallbacks<R, F> for NoOp {
    type ScanEvent = ();

    fn on_numeric(
        &mut self,
        _field: F,
        _value: NumericField,
    ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
        Ok(())
    }

    fn on_group(
        &mut self,
        _field: F,
        _op: GroupOp,
    ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _field: F,
        _delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
        Ok(())
    }
}

impl ResettableScanner for NoOp {
    fn reset(&mut self) {}
}

impl IntoResettableScanner for NoOp {
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
