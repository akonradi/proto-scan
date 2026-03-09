use crate::read::ReadTypes;
use crate::scan::{IntoScanOutput, ScanError};
#[cfg(doc)]
use crate::wire::FieldNumber;
use crate::wire::{GroupOp, LengthDelimited, NumericField};

mod message;
mod no_op;
mod save;
mod write;

pub use message::Message;
pub use no_op::NoOp;
pub use save::Save;
pub use write::Write;

// Not exported, only available for use in maps.
pub(super) use save::{SaveBytesScanner, SaveNumeric};

/// Implemented by a visitor for a fixed [`FieldNumber`].
pub trait OnScanField<R: ReadTypes>: IntoScanOutput {
    type ScanEvent;

    /// Called when a numeric tag is read.
    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>>;

    /// Called when a SGROUP or EGROUP tag is read.
    fn on_group(&mut self, op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>>;

    /// Called when a length-delimited tag is read.
    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>>;
}
