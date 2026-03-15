#![doc(hidden)]
use core::convert::Infallible;

use crate::read::{ReadBuffer, ReadTypes};
use crate::scan::field::OnScanField;
use crate::scan::{IntoScanOutput, Resettable, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField, WrongWireType};

/// [`OnScanField`] impl that produces the read value as the event output.
pub struct SaveBytesScanner<E: DecodeFromBytes + ?Sized, R: ReadTypes>(E::Decoded<R>);

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> SaveBytesScanner<E, R> {
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> Clone for SaveBytesScanner<E, R>
where
    E::Decoded<R>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> Default for SaveBytesScanner<E, R> {
    fn default() -> Self {
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
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let bytes = delimited.into_bytes()?;
        self.0 = T::decode(bytes).map_err(|_| ScanError::Utf8)?;
        Ok(None)
    }
}

impl<T: DecodeFromBytes + ?Sized, R: ReadTypes> IntoScanOutput for SaveBytesScanner<T, R> {
    type ScanOutput = T::Decoded<R>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> Resettable for SaveBytesScanner<E, R> {
    fn reset(&mut self) {
        self.0 = Default::default();
    }
}
