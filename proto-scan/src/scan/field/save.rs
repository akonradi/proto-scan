use std::convert::Infallible;
use std::marker::PhantomData;

use crate::read::ReadTypes;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{IntoResettable, IntoScanOutput, IntoScanner, Resettable, StopScan};
use crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct SaveNumeric<E: Encoding>(Option<E::Repr>);

impl<T: Encoding> SaveNumeric<T> {
    pub fn new() -> Self {
        Self(None)
    }
}

impl<E: Encoding, R: ReadTypes> OnScanField<R> for SaveNumeric<E> {
    type ScanEvent = E::Repr;

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
        let value = E::decode(value).map_err(Into::into)?;
        self.0 = Some(value);
        Ok(Some(value))
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }
}

impl<E: Encoding> Resettable for SaveNumeric<E> {
    fn reset(&mut self) {
        self.0 = None;
    }
}

impl<E: Encoding> IntoResettable for SaveNumeric<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl<E: Encoding> IntoScanner for SaveNumeric<E> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

impl<E: Encoding> IntoScanOutput for SaveNumeric<E> {
    type ScanOutput = Option<E::Repr>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct SaveRepeated<E: Encoding>(Vec<E::Repr>);

impl<T: Encoding> SaveRepeated<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<E: Encoding> IntoScanOutput for SaveRepeated<E> {
    type ScanOutput = Vec<E::Repr>;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: Encoding, R: ReadTypes> OnScanField<R> for SaveRepeated<E> {
    type ScanEvent = Infallible;
    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
        self.0.extend([E::decode(value).map_err(Into::into)?]);
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        let mut packed = delimited.into_packed::<E::Wire>();
        let mut result = Ok(None);
        self.0.extend(core::iter::from_fn(|| {
            let value = packed.next()?.ok().and_then(|w| E::decode(w).ok());
            if value.is_none() {
                result = Err(StopScan);
            }
            value
        }));
        result
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }
}

impl<E: Encoding> Resettable for SaveRepeated<E> {
    fn reset(&mut self) {
        self.0.clear()
    }
}

impl<E: Encoding> IntoScanner for SaveRepeated<E> {
    type Scanner<R: ReadTypes> = Self;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        self
    }
}

/// [`IntoScanner`] implementation whose `Scanner` type produces the read value as the event output.
pub struct SaveBytes<E: DecodeFromBytes + ?Sized>(PhantomData<E>);

/// [`OnScanField`] impl that produces the read value as the event output.
pub struct SaveBytesScanner<E: DecodeFromBytes + ?Sized, R: ReadTypes>(Option<E::Decoded<R>>);

pub trait DecodeFromBytes {
    type Error;
    type Decoded<R: ReadTypes>;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error>;
}

impl DecodeFromBytes for str {
    type Error = core::str::Utf8Error;
    type Decoded<R: ReadTypes> = String;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error> {
        str::from_utf8(bytes.as_ref()).map(Into::into)
    }
}

impl DecodeFromBytes for [u8] {
    type Error = Infallible;
    type Decoded<R: ReadTypes> = Box<[u8]>;
    fn decode<R: ReadTypes>(bytes: R::Buffer) -> Result<Self::Decoded<R>, Self::Error> {
        Ok(bytes.as_ref().into())
    }
}

impl<T: DecodeFromBytes + ?Sized> SaveBytes<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: DecodeFromBytes + ?Sized> IntoScanner for SaveBytes<T> {
    type Scanner<R: ReadTypes> = SaveBytesScanner<T, R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner(None)
    }
}

impl<T: DecodeFromBytes + ?Sized, R: ReadTypes> OnScanField<R> for SaveBytesScanner<T, R> {
    type ScanEvent = Infallible;
    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        let bytes = delimited.into_bytes().map_err(|_| StopScan)?;
        let decoded = T::decode(bytes).map_err(|_| StopScan)?;
        self.0 = Some(decoded);
        Ok(None)
    }
}

impl<T: DecodeFromBytes + ?Sized, R: ReadTypes> IntoScanOutput for SaveBytesScanner<T, R> {
    type ScanOutput = Option<T::Decoded<R>>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> Resettable for SaveBytesScanner<E, R> {
    fn reset(&mut self) {
        self.0 = None;
    }
}
