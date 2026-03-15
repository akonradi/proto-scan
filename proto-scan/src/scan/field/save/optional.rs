#![doc(hidden)]

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{IntoScanOutput, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField};

#[derive(Clone, Default)]
pub struct SaveOptional<S> {
    pub(crate) inner: S,
    pub(crate) present: bool,
}

impl<S> SaveOptional<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            present: false,
        }
    }
}

impl<S: IntoScanOutput> IntoScanOutput for SaveOptional<S> {
    type ScanOutput = Option<S::ScanOutput>;
    fn into_scan_output(self) -> Self::ScanOutput {
        let Self { inner, present } = self;
        present.then(|| inner.into_scan_output())
    }
}

impl<S: OnScanField<R>, R: ReadTypes> OnScanField<R> for SaveOptional<S> {
    type ScanEvent = S::ScanEvent;

    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let event = self.inner.on_numeric(value)?;
        self.present = true;
        Ok(event)
    }

    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let event = self.inner.on_group(op)?;
        self.present = true;
        Ok(event)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let event = self.inner.on_length_delimited(delimited)?;
        self.present = true;
        Ok(event)
    }
}
