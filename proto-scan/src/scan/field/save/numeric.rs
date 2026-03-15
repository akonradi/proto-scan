#![doc(hidden)]

use crate::read::ReadTypes;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{IntoResettable, IntoScanOutput, Resettable, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType, WrongWireType};

pub struct SaveNumeric<E: Encoding>(E::Repr);

impl<E: Encoding> SaveNumeric<E> {
    pub(super) fn new() -> Self {
        Self(Default::default())
    }
}

impl<E: Encoding> Clone for SaveNumeric<E> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<E: Encoding> Default for SaveNumeric<E> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<E: Encoding, R: ReadTypes> OnScanField<R> for SaveNumeric<E> {
    type ScanEvent = E::Repr;

    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let value = <E::Wire as NumericWireType>::from_value(value)?;
        let value = E::decode(value).map_err(Into::into)?;
        self.0 = value;
        Ok(Some(value))
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

impl<E: Encoding> Resettable for SaveNumeric<E> {
    fn reset(&mut self) {
        self.0 = Default::default();
    }
}

impl<E: Encoding> IntoResettable for SaveNumeric<E> {
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
