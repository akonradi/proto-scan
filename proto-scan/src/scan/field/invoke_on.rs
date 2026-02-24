use std::marker::PhantomData;

use crate::scan::Resettable;
use crate::scan::field::OnScanField;
use crate::scan::{ScanTypes, StopScan};
use crate::wire::{GroupOp, LengthDelimited, ScalarField, ScalarWireType};
use core::convert::Infallible;

/// Invokes the provided callback for each scalar value.
///
/// [`OnScanField::on_scalar`] returns an error if the encoded value has the
/// wrong wire type.
pub struct InvokeOn<W, F>(F, PhantomData<W>);

impl<'a, W: ScalarWireType, F: FnMut(W::Repr) -> Result<(), StopScan>> ScanTypes
    for InvokeOn<W, F>
{
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'a, W: ScalarWireType, F: FnMut(W::Repr) -> Result<(), StopScan>> OnScanField
    for InvokeOn<W, F>
{
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Infallible>, StopScan> {
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

impl<W, F> Resettable for InvokeOn<W, F> {
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {}
    fn reset(&mut self, (): Self::Mark) {}
}
