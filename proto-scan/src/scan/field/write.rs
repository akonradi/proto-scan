use core::convert::Infallible;
use core::marker::PhantomData;
use core::ops::DerefMut;

use crate::read::{BoundsOnlyReadTypes, ReadTypes};
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::save::DecodeFromBytes;
use crate::scan::field::{OnScanField, Repeated};
use crate::scan::save_from::SaveFrom;
use crate::scan::{GroupOp, IntoScanOutput, IntoScanner, NumericField, ScanError};
use crate::scan::{IntoResettableScanner, ResettableScanner};
use crate::wire::{LengthDelimited, NumericWireType, WrongWireType};

pub struct Write<T>(pub T);

macro_rules! impl_into_scanner {
    ($p:path) => {
        impl<'t, T> IntoScanner<$p> for Write<&'t mut T>
        where
            <$p as Encoding>::Repr: Into<T>,
        {
            type Scanner<R: ReadTypes> = WriteNumeric<$p, &'t mut T>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                WriteNumeric(self.0, PhantomData)
            }
        }

        impl<D> IntoScanner<Repeated<$p>> for Write<D>
        where
            WriteRepeated<$p, D>: OnScanField<BoundsOnlyReadTypes>,
        {
            type Scanner<R: ReadTypes> = WriteRepeated<$p, D>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                WriteRepeated(self.0, PhantomData)
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

impl<T> IntoScanner<[u8]> for Write<T> {
    type Scanner<R: ReadTypes> = WriteBytes<[u8], T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteBytes::new(self.0)
    }
}
impl<T> IntoScanner<str> for Write<T> {
    type Scanner<R: ReadTypes> = WriteBytes<str, T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteBytes::new(self.0)
    }
}

/// [`OnScanField`] that writes the decoded value to the provided location.
pub struct WriteNumeric<E, D>(D, PhantomData<E>);

impl<E: Encoding, D: SaveFrom<E::Repr>, R: ReadTypes> OnScanField<R> for WriteNumeric<E, D> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        let value = <E::Wire as NumericWireType>::from_value(value)?;
        self.0.save_from(E::decode(value).map_err(Into::into)?);
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

impl<E: Encoding, D> IntoScanOutput for WriteNumeric<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

/// Implements [`SaveFrom`] and [`Resettable`] to save and restore a previous value.
pub struct RestoreOnReset<'t, D>(&'t mut D, Option<D>);

impl<'t, D> ResettableScanner for RestoreOnReset<'t, D> {
    fn reset(&mut self) {
        if let Some(prev) = self.1.take() {
            *self.0 = prev;
        }
    }
}

impl<'t, T: Into<D>, D> SaveFrom<T> for RestoreOnReset<'t, D> {
    fn save_from(&mut self, value: T) {
        self.1 = Some(core::mem::replace(self.0, value.into()));
    }
}

impl<'t, E, D> IntoResettableScanner for WriteNumeric<E, &'t mut D> {
    type Resettable = WriteNumeric<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        WriteNumeric(RestoreOnReset(self.0, None), PhantomData)
    }
}

impl<'t, E, D> ResettableScanner for WriteNumeric<E, RestoreOnReset<'t, D>> {
    fn reset(&mut self) {
        self.0.reset();
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteRepeated<E, D>(D, PhantomData<E>);

impl<E: Encoding, R: ReadTypes, D: DerefMut<Target: Extend<E::Repr>>> OnScanField<R>
    for WriteRepeated<E, D>
{
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        value: NumericField,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        let value = <E::Wire as NumericWireType>::from_value(value)?;
        let decoded = E::decode(value).map_err(Into::into)?;
        self.0.extend([decoded]);
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        let mut packed = delimited.into_packed::<E::Wire>();
        let mut result = Ok(None);
        self.0.extend(core::iter::from_fn(|| {
            let value = packed
                .next()?
                .map_err(ScanError::from)
                .and_then(|w| E::decode(w).map_err(|e| ScanError::from(e.into())));
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
}

impl<E: Encoding, D> IntoScanOutput for WriteRepeated<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

#[cfg(feature = "std")]
impl<'t, E, D> IntoResettableScanner for WriteRepeated<E, &'t mut Vec<D>> {
    type Resettable = WriteRepeated<E, RestoreLenOnReset<'t, Vec<D>>>;
    fn into_resettable(self) -> Self::Resettable {
        WriteRepeated(RestoreLenOnReset::new(self.0), PhantomData)
    }
}

#[cfg(feature = "std")]
pub struct RestoreLenOnReset<'t, T>(&'t mut T, usize);

#[cfg(feature = "std")]
impl<'t, T> RestoreLenOnReset<'t, Vec<T>> {
    pub fn new(arg: &'t mut Vec<T>) -> Self {
        let len = arg.len();
        Self(arg, len)
    }
}

#[cfg(feature = "std")]
impl<'t, T> ResettableScanner for RestoreLenOnReset<'t, Vec<T>> {
    fn reset(&mut self) {
        self.0.truncate(self.1);
    }
}

#[cfg(feature = "std")]
impl<'t, T> std::ops::Deref for RestoreLenOnReset<'t, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(feature = "std")]
impl<'t, T> DerefMut for RestoreLenOnReset<'t, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<D: ResettableScanner, E> ResettableScanner for WriteRepeated<E, D> {
    fn reset(&mut self) {
        self.0.reset();
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteBytes<E: ?Sized, D>(D, PhantomData<E>);

impl<E: ?Sized, D> WriteBytes<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<B: DecodeFromBytes + ?Sized, D: SaveFrom<B::Decoded<R>>, R: ReadTypes> OnScanField<R>
    for WriteBytes<B, D>
{
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        let bytes = delimited.into_bytes()?;
        let decoded = B::decode(bytes).map_err(|_| ScanError::Utf8)?;
        self.0.save_from(decoded);
        Ok(None)
    }
}

impl<S: ?Sized, D> IntoScanOutput for WriteBytes<S, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<D: ResettableScanner, E: ?Sized> ResettableScanner for WriteBytes<E, D> {
    fn reset(&mut self) {
        self.0.reset();
    }
}

impl<'t, E: ?Sized, D> IntoResettableScanner for WriteBytes<E, &'t mut D> {
    type Resettable = WriteBytes<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        WriteBytes(RestoreOnReset(self.0, None), PhantomData)
    }
}
