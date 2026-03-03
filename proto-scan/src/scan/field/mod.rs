use crate::read::ReadTypes;
use crate::scan::{IntoScanOutput, StopScan};
#[cfg(doc)]
use crate::wire::FieldNumber;
use crate::wire::{GroupOp, LengthDelimited, NumericField};

mod invoke_on;
mod message;
mod no_op;
mod save;
mod write;

pub use invoke_on::InvokeOn;
pub use message::Message;
pub use no_op::NoOp;
#[cfg(feature = "std")]
pub use save::SaveRepeated;
pub use save::{SaveBytes, SaveNumeric};
pub use write::{SaveFrom, WriteBytes, WriteNumeric, WriteRepeated};

/// Implemented by a visitor for a fixed [`FieldNumber`].
pub trait OnScanField<R: ReadTypes>: IntoScanOutput {
    type ScanEvent;

    /// Called when a numeric tag is read.
    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan>;

    /// Called when a SGROUP or EGROUP tag is read.
    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan>;

    /// Called when a length-delimited tag is read.
    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, StopScan>;
}
