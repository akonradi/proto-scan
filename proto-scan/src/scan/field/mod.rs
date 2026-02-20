use crate::scan::StopScan;
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
pub trait OnScanField {
    type ScanEvent;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan>;

    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan>;

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan>;
}
