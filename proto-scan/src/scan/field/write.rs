use core::convert::Infallible;
use core::marker::PhantomData;
use core::ops::DerefMut;

use crate::read::{BoundsOnlyReadTypes, ReadTypes};
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, IntoScanOutput, IntoScanner, NumericField, Repeated, StopScan};
use crate::scan::{IntoResettable, Resettable};
use crate::wire::{LengthDelimited, NumericWireType};

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

/// Generalization of assignment for wrapping types.
///
/// A type `Wrap<T>(T, bool)` could implement `SaveFrom<T>` to save the value
/// and note that a modification was made.
pub trait SaveFrom<T> {
    fn save_from(&mut self, value: T);
}

impl<S, T: From<S>> SaveFrom<S> for &mut T {
    fn save_from(&mut self, value: S) {
        **self = value.into();
    }
}

impl<E: Encoding, D: SaveFrom<E::Repr>, R: ReadTypes> OnScanField<R> for WriteNumeric<E, D> {
    type ScanEvent = Infallible;

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
        self.0.save_from(E::decode(value).map_err(Into::into)?);
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }
}

impl<E: Encoding, D> IntoScanOutput for WriteNumeric<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

/// Implements [`SaveFrom`] and [`Resettable`] to save and restore a previous value.
pub struct RestoreOnReset<'t, D>(&'t mut D, Option<D>);

impl<'t, D> Resettable for RestoreOnReset<'t, D> {
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

impl<'t, E, D> IntoResettable for WriteNumeric<E, &'t mut D> {
    type Resettable = WriteNumeric<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        WriteNumeric(RestoreOnReset(self.0, None), PhantomData)
    }
}

impl<'t, E, D> Resettable for WriteNumeric<E, RestoreOnReset<'t, D>> {
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

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
        let decoded = E::decode(value).map_err(Into::into)?;
        self.0.extend([decoded]);
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Infallible>, StopScan> {
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
}

impl<E: Encoding, D> IntoScanOutput for WriteRepeated<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

#[cfg(feature = "std")]
impl<'t, E, D> IntoResettable for WriteRepeated<E, &'t mut Vec<D>> {
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
impl<'t, T> Resettable for RestoreLenOnReset<'t, Vec<T>> {
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

impl<D: Resettable, E> Resettable for WriteRepeated<E, D> {
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

impl<D: for<'a> SaveFrom<&'a [u8]>, R: ReadTypes> OnScanField<R> for WriteBytes<[u8], D> {
    type ScanEvent = Infallible;

    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Infallible>, StopScan> {
        let bytes = delimited.into_bytes().ok().ok_or(StopScan)?;
        self.0.save_from(bytes.as_ref());
        Ok(None)
    }
}

impl<D: for<'a> SaveFrom<&'a str>, R: ReadTypes> OnScanField<R> for WriteBytes<str, D> {
    type ScanEvent = Infallible;
    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Infallible>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Infallible>, StopScan> {
        let bytes = delimited.into_bytes().ok().ok_or(StopScan)?;
        let bytes = core::str::from_utf8(bytes.as_ref()).map_err(|_| StopScan)?;
        self.0.save_from(bytes);
        Ok(None)
    }
}

impl<S: ?Sized, D> IntoScanOutput for WriteBytes<S, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<D: Resettable, E: ?Sized> Resettable for WriteBytes<E, D> {
    fn reset(&mut self) {
        self.0.reset();
    }
}

impl<'t, E: ?Sized, D> IntoResettable for WriteBytes<E, &'t mut D> {
    type Resettable = WriteBytes<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        WriteBytes(RestoreOnReset(self.0, None), PhantomData)
    }
}
