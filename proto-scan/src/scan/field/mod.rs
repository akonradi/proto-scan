use crate::scan::{ScanTypes, StopScan};
#[cfg(doc)]
use crate::wire::FieldNumber;
use crate::wire::{GroupOp, LengthDelimited, ScalarField};

mod emit_scalar;
mod invoke_on;
mod no_op;
mod save;

pub use emit_scalar::EmitScalar;
pub use invoke_on::InvokeOn;
pub use no_op::NoOp;
pub use save::Save;

/// Implemented by a visitor for a fixed [`FieldNumber`].
pub trait OnScanField: ScanTypes<ScanOutput: Default> {
    fn into_output(self) -> Self::ScanOutput;

    /// Called when a scalar tag is read.
    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan>;

    /// Called when a SGROUP or EGROUP tag is read.
    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan>;

    /// Called when a length-delimited tag is read.
    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan>;
}
