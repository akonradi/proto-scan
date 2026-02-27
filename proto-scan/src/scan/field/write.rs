use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, IntoScanner, NumericField, ScanTypes, StopScan};
use crate::scan::{IntoResettable, Resettable};
use crate::wire::{LengthDelimited, NumericWireType};

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

impl<E, D> WriteNumeric<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E, D> ScanTypes for WriteNumeric<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<E: Encoding, D: SaveFrom<E::Repr>> OnScanField for WriteNumeric<E, D> {
    fn into_output(self) -> Self::ScanOutput {}

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

/// Implements [`WriteFrom`] and [`Resettable`] to save and restore a previous value.
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
        self.1 = Some(core::mem::replace(&mut self.0, value.into()));
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

impl<E, D> IntoScanner for WriteNumeric<E, D> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteRepeated<E, D>(D, PhantomData<E>);

impl<E, D> WriteRepeated<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E: Encoding, D> ScanTypes for WriteRepeated<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'t, E: Encoding, D: DerefMut<Target: Extend<E::Repr>>> OnScanField for WriteRepeated<E, D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
        let decoded = E::decode(value).map_err(Into::into)?.into();
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

impl<'t, E, D> IntoResettable for WriteRepeated<E, &'t mut Vec<D>> {
    type Resettable = WriteRepeated<E, RestoreLenOnReset<'t, Vec<D>>>;
    fn into_resettable(self) -> Self::Resettable {
        WriteRepeated(RestoreLenOnReset::new(self.0), PhantomData)
    }
}

pub struct RestoreLenOnReset<'t, T>(&'t mut T, usize);

impl<'t, T> RestoreLenOnReset<'t, Vec<T>> {
    pub fn new(arg: &'t mut Vec<T>) -> Self {
        let len = arg.len();
        Self(arg, len)
    }
}

impl<'t, T> Resettable for RestoreLenOnReset<'t, Vec<T>> {
    fn reset(&mut self) {
        self.0.truncate(self.1);
    }
}

impl<'t, T> Deref for RestoreLenOnReset<'t, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

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

impl<E, D> IntoScanner for WriteRepeated<E, D> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteBytes<E: ?Sized, D>(D, PhantomData<E>);

impl<E: ?Sized, D> WriteBytes<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E: ?Sized, D> ScanTypes for WriteBytes<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<D: for<'a> SaveFrom<&'a [u8]>> OnScanField for WriteBytes<[u8], D> {
    fn into_output(self) -> Self::ScanOutput {}

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
        self.0.save_from(bytes.as_ref().into());
        Ok(None)
    }
}

impl<D: for<'a> SaveFrom<&'a str>> OnScanField for WriteBytes<str, D> {
    fn into_output(self) -> Self::ScanOutput {}

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
        self.0.save_from(bytes.into());
        Ok(None)
    }
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

impl<E: ?Sized, D> IntoScanner for WriteBytes<E, D> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}
