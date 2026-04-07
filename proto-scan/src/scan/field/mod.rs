use crate::read::ReadTypes;
use crate::scan::{GroupDelimited, IntoScanOutput, ScanCallbacks, ScanError, ScanLengthDelimited};
#[cfg(doc)]
use crate::wire::FieldNumber;
use crate::wire::NumericField;

mod group;
pub mod map;
mod message;
mod no_op;
mod repeated;
mod save;
mod write;

pub use group::Group;
pub use map::{Map, MapKey};
pub use message::{Message, ScanOptionalMessage};
pub use no_op::NoOp;
pub use repeated::{
    Fold, RepeatStrategy, RepeatStrategyScanner, Repeated, ScanRepeated, WriteCloned,
};
pub use save::{Save, SaveCloned};
pub use write::{SaveFrom, Write};

/// Implemented by a visitor for a fixed [`FieldNumber`].
pub trait OnScanField<R: ReadTypes>: IntoScanOutput {
    /// Called when a numeric tag is read.
    fn on_numeric(&mut self, value: NumericField) -> Result<(), ScanError<R::Error>>;

    /// Called when a SGROUP tag is read.
    fn on_group(
        &mut self,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>>;

    /// Called when a length-delimited tag is read.
    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>>;
}

#[cfg(feature = "std")]
impl<S: OnScanField<R>, R: ReadTypes> OnScanField<R> for Box<S> {
    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_numeric(&mut *self, value)
    }

    fn on_group(
        &mut self,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_group(&mut *self, group)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_length_delimited(&mut *self, delimited)
    }
}

/// Utility type for wrapping a [`ScanCallbacks`] impl into an [`IntoScanOutput<ScanOutput=()>`]
struct NoOutput<S>(S);

impl<S> IntoScanOutput for NoOutput<S> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<R: ReadTypes, S: ScanCallbacks<R>> ScanCallbacks<R> for NoOutput<S> {
    fn on_numeric(
        &mut self,
        field: crate::wire::FieldNumber,
        value: crate::wire::NumericField,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_numeric(&mut self.0, field, value)
    }

    fn on_group(
        &mut self,
        field: crate::wire::FieldNumber,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_group(&mut self.0, field, group)
    }

    fn on_length_delimited(
        &mut self,
        field: crate::wire::FieldNumber,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        S::on_length_delimited(&mut self.0, field, delimited)
    }
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
