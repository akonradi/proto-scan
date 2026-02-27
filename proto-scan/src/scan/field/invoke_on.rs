use std::marker::PhantomData;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{IntoScanOutput, StopScan};
use crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType};
use core::convert::Infallible;

/// Invokes the provided callback for each numeric value.
///
/// [`OnScanField::on_numeric`] returns an error if the encoded value has the
/// wrong wire type.
pub struct InvokeOn<W, F>(F, PhantomData<W>);

impl<'a, W: NumericWireType, R: ReadTypes, F: FnMut(W::Repr) -> Result<(), StopScan>> OnScanField<R>
    for InvokeOn<W, F>
{
    type ScanEvent = Infallible;

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Infallible>, StopScan> {
        let value = W::from_value(value).ok_or(StopScan)?;
        let () = (self.0)(value)?;
        Ok(None)
    }
    fn on_group(&mut self, _: GroupOp) -> Result<Option<Infallible>, StopScan> {
        Ok(None)
    }
    fn on_length_delimited(
        &mut self,
        _: impl LengthDelimited,
    ) -> Result<Option<Infallible>, StopScan> {
        Ok(None)
    }
}

impl<W, F> IntoScanOutput for InvokeOn<W, F> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}
