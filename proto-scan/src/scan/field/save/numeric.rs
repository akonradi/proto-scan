#![doc(hidden)]

use derive_where::derive_where;

use crate::read::ReadTypes;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoResettableScanner, IntoScanOutput, ResettableScanner, ScanError,
    ScanLengthDelimited,
};
use crate::wire::{NumericField, NumericWireType, WrongWireType};

#[derive_where(Default, Clone; E::Repr)]
pub struct SaveNumeric<E: Encoding>(E::Repr);

impl<E: Encoding, R: ReadTypes> OnScanField<R> for SaveNumeric<E> {
    fn on_numeric(&mut self, value: NumericField) -> Result<(), ScanError<R::Error>> {
        let value = <E::Wire as NumericWireType>::from_value(value)?;
        self.0 = E::decode(value).map_err(Into::into)?;
        Ok(())
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl ScanLengthDelimited,
    ) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

impl<E: Encoding> ResettableScanner for SaveNumeric<E> {
    fn reset(&mut self) {
        self.0 = Default::default();
    }
}

impl<E: Encoding> IntoResettableScanner for SaveNumeric<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl<E: Encoding> IntoScanOutput for SaveNumeric<E> {
    type ScanOutput = E::Repr;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}
