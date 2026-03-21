#![doc(hidden)]

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{GroupDelimited, IntoScanOutput, ScanError, ScanLengthDelimited};
use crate::wire::NumericField;

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
    fn on_numeric(&mut self, value: NumericField) -> Result<(), ScanError<R::Error>> {
        self.inner.on_numeric(value)?;
        self.present = true;
        Ok(())
    }

    fn on_group(
        &mut self,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        self.inner.on_group(group)?;
        self.present = true;
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        self.inner.on_length_delimited(delimited)?;
        self.present = true;
        Ok(())
    }
}
