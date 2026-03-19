use crate::read::ReadTypes;
use crate::scan::{IntoScanOutput, ScanError, ScanLengthDelimited};
#[cfg(doc)]
use crate::wire::FieldNumber;
use crate::wire::{GroupOp, NumericField};

mod map;
mod message;
mod no_op;
mod repeated;
mod save;
mod write;

pub use map::{Map, MapKey};
pub use message::Message;
pub use no_op::NoOp;
pub use repeated::{
    Fold, RepeatStrategy, RepeatStrategyScanner, Repeated, ScanRepeated, WriteCloned,
};
pub use save::{Save, SaveCloned};
pub use write::Write;

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
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>>;
}

#[cfg(test)]
mod test {
    macro_rules! assert_impl_into_scanner {
        ($t:ty: IntoScanner<$p:ty>) => {
            static_assertions::assert_impl_all! ($t: $crate::scan::IntoScanner<$p>);
            static_assertions::assert_impl_all! (
                <$t as $crate::scan::IntoScanner<$p>>::Scanner<$crate::read::BoundsOnlyReadTypes>:
                    $crate::scan::field::OnScanField<$crate::read::BoundsOnlyReadTypes>,
                    $crate::scan::IntoScanOutput,
            );
        };
        ($t:ty: IntoScanner<$p:ty>; resettable) => {
            assert_impl_into_scanner!($t: IntoScanner<$p>);
            static_assertions::assert_impl_all! (
                <$t as $crate::scan::IntoScanner<$p>>::Scanner<$crate::read::BoundsOnlyReadTypes>:
                    $crate::scan::IntoResettableScanner
            );
        };
    }

    pub(crate) use assert_impl_into_scanner;
}
