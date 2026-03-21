use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, MessageScanner, ResettableScanner, ScanCallbacks,
    ScanError, ScanLengthDelimited,
};
use crate::wire::WrongWireType;

/// Wrapper type for a proto2 group scanner.
#[derive(Clone)]
pub struct Group<F>(F);

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Group<M>> for S {
    type Scanner<R: ReadTypes> = Group<S::Scanner<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Group(S::into_scanner(self))
    }
}

impl<F: ScanCallbacks<R> + IntoScanOutput, R: ReadTypes> OnScanField<R> for Group<F> {
    fn on_numeric(&mut self, _value: crate::scan::NumericField) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        delimited: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        delimited.scan_with(&mut self.0)?;
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

impl<F: IntoScanOutput> IntoScanOutput for Group<F> {
    type ScanOutput = F::ScanOutput;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0.into_scan_output()
    }
}

impl<F: ResettableScanner> ResettableScanner for Group<F> {
    fn reset(&mut self) {
        self.0.reset()
    }
}
