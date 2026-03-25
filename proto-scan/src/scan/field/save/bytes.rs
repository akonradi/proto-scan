#![doc(hidden)]
use core::convert::Infallible;

use derive_where::derive_where;

use crate::read::{ReadBuffer, ReadTypes};
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoResettableScanner, IntoScanOutput, ResettableScanner, ScanError,
    ScanLengthDelimited,
};
use crate::wire::{NumericField, WrongWireType};

/// [`OnScanField`] impl that produces the read value as the event output.
#[derive_where(Debug, Clone, Default; E::Decoded<R>)]
pub struct SaveBytesScanner<E: DecodeFromBytes + ?Sized, R: ReadTypes>(E::Decoded<R>);

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> SaveBytesScanner<E, R> {
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }
}

pub trait DecodeFromBytes {
    type Error;
    type Decoded<R: ReadTypes>: Default;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error>;
}

impl DecodeFromBytes for str {
    type Error = core::str::Utf8Error;
    type Decoded<R: ReadTypes> = <R::Buffer as ReadBuffer>::String;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error> {
        bytes.into_string()
    }
}

impl DecodeFromBytes for [u8] {
    type Error = Infallible;
    type Decoded<R: ReadTypes> = R::Buffer;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error> {
        Ok(bytes)
    }
}

impl<T: DecodeFromBytes + ?Sized, R: ReadTypes> OnScanField<R> for SaveBytesScanner<T, R> {
    fn on_numeric(&mut self, _value: NumericField) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        let bytes = delimited.into_bytes()?;
        self.0 = T::decode(bytes).map_err(|_| ScanError::Utf8)?;
        Ok(())
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> IntoResettableScanner for SaveBytesScanner<E, R> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl<T: DecodeFromBytes + ?Sized, R: ReadTypes> IntoScanOutput for SaveBytesScanner<T, R> {
    type ScanOutput = T::Decoded<R>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> ResettableScanner for SaveBytesScanner<E, R> {
    fn reset(&mut self) {
        self.0 = Default::default();
    }
}
