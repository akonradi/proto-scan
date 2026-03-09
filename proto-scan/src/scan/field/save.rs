use core::convert::Infallible;

use crate::read::{ReadBuffer, ReadTypes};
#[cfg(feature = "std")]
use crate::scan::Repeated;
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::OnScanField;
use crate::scan::{IntoResettable, IntoScanOutput, IntoScanner, Resettable, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType, WrongWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct Save;

macro_rules! impl_into_scanner {
    ($p:path) => {
        impl IntoScanner<$p> for Save {
            type Scanner<R: ReadTypes> = SaveNumeric<$p>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                SaveNumeric(None)
            }
        }
        #[cfg(feature = "std")]
        impl IntoScanner<Repeated<$p>> for Save {
            type Scanner<R: ReadTypes> = SaveRepeated<$p>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                SaveRepeated(Vec::new())
            }
        }
    };
}

impl_into_scanner!(Varint<bool>);
impl_into_scanner!(Varint<i32>);
impl_into_scanner!(Varint<i64>);
impl_into_scanner!(Varint<u32>);
impl_into_scanner!(Varint<u64>);
impl_into_scanner!(Varint<ZigZag<i32>>);
impl_into_scanner!(Varint<ZigZag<i64>>);
impl_into_scanner!(Fixed<u64>);
impl_into_scanner!(Fixed<u32>);
impl_into_scanner!(Fixed<i64>);
impl_into_scanner!(Fixed<i32>);
impl_into_scanner!(Fixed<f64>);
impl_into_scanner!(Fixed<f32>);

impl IntoScanner<str> for Save {
    type Scanner<R: ReadTypes> = SaveBytesScanner<str, R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner(None)
    }
}

impl IntoScanner<[u8]> for Save {
    type Scanner<R: ReadTypes> = SaveBytesScanner<[u8], R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner(None)
    }
}

pub struct SaveNumeric<E: Encoding>(Option<E::Repr>);

impl<E: Encoding> Clone for SaveNumeric<E> {
    fn clone(&self) -> Self {
        Self(self.0)
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
        self.0 = Some(value);
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
        self.0 = None;
    }
}

impl<E: Encoding> IntoResettable for SaveNumeric<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl<E: Encoding> IntoScanOutput for SaveNumeric<E> {
    type ScanOutput = Option<E::Repr>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

#[cfg(feature = "std")]
/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct SaveRepeated<E: Encoding>(Vec<E::Repr>);

#[cfg(feature = "std")]
impl<E: Encoding> IntoScanOutput for SaveRepeated<E> {
    type ScanOutput = Vec<E::Repr>;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

#[cfg(feature = "std")]
impl<E: Encoding, R: ReadTypes> OnScanField<R> for SaveRepeated<E> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let value = <E::Wire as NumericWireType>::from_value(value)?;
        self.0.extend([E::decode(value).map_err(Into::into)?]);
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let mut packed = delimited.into_packed::<E::Wire>();
        let mut result = Ok(None);
        self.0.extend(core::iter::from_fn(|| {
            let value = packed
                .next()?
                .map_err(ScanError::from)
                .and_then(|w| Ok(E::decode(w).map_err(Into::into)?));
            match value {
                Err(e) => {
                    result = Err(e);
                    None
                }
                Ok(v) => Some(v),
            }
        }));
        result
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

#[cfg(feature = "std")]
impl<E: Encoding> IntoResettable for SaveRepeated<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

#[cfg(feature = "std")]
impl<E: Encoding> Resettable for SaveRepeated<E> {
    fn reset(&mut self) {
        self.0.clear()
    }
}

/// [`OnScanField`] impl that produces the read value as the event output.
pub struct SaveBytesScanner<E: DecodeFromBytes + ?Sized, R: ReadTypes>(Option<E::Decoded<R>>);

impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> Clone for SaveBytesScanner<E, R>
where
    E::Decoded<R>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub trait DecodeFromBytes {
    type Error;
    type Decoded<R: ReadTypes>;
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
        let decoded = T::decode(bytes).map_err(|_| ScanError::Utf8)?;
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
