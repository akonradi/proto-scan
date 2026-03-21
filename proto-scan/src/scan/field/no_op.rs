use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoResettableScanner, IntoScanOutput, IntoScanner, NumericField,
    ResettableScanner, ScanCallbacks, ScanError, ScanLengthDelimited,
};

/// [`OnScanField`] impl that does nothing and always succeeds.
#[derive(Copy, Clone, Default)]
pub struct NoOp;

impl<R: ReadTypes> OnScanField<R> for NoOp {
    fn on_numeric(&mut self, _value: NumericField) -> Result<(), ScanError<R::Error>> {
        Ok(())
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl ScanLengthDelimited,
    ) -> Result<(), ScanError<R::Error>> {
        Ok(())
    }
}

impl<R: ReadTypes, F> ScanCallbacks<R, F> for NoOp {
    fn on_numeric(&mut self, _field: F, _value: NumericField) -> Result<(), ScanError<R::Error>> {
        Ok(())
    }

    fn on_group(
        &mut self,
        _field: F,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _field: F,
        _delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
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
