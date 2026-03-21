use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, MessageScanner, ResettableScanner, ScanCallbacks,
    ScanError, ScanLengthDelimited,
};
use crate::wire::WrongWireType;

#[derive(Clone)]
pub struct Group<F>(F);

impl<F> Group<F> {
    pub fn new(scanner: F) -> Self {
        Self(scanner)
    }
}

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Group<M>> for S {
    type Scanner<R: ReadTypes> = Group<S::Scanner<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Group::new(S::into_scanner(self))
    }
}

impl<F: ScanCallbacks<R> + IntoScanOutput, R: ReadTypes> OnScanField<R> for Group<F> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: crate::scan::NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        delimited: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        delimited.scan_with(&mut self.0)?;
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
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
