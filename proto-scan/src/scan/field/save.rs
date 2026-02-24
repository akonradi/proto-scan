use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, ScalarField, ScanTypes, StopScan};
use crate::scan::{IntoResettable, Resettable};
use crate::wire::{LengthDelimited, ScalarWireType};

/// [`OnScanField`] that writes the decoded value to the provided location.
pub struct SaveScalar<E, D>(D, PhantomData<E>);

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

impl<E, D> SaveScalar<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E, D> ScanTypes for SaveScalar<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<E: Encoding, D: SaveFrom<E::Repr>> OnScanField for SaveScalar<E, D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
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

/// Implements [`SaveFrom`] and [`Resettable`] to save and restore a previous value.
pub struct RestoreOnReset<'t, D>(&'t mut D, Option<D>);

impl<'t, D> Resettable for RestoreOnReset<'t, D> {
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {}
    fn reset(&mut self, (): Self::Mark) {
        if let Some(prev) = self.1.take() {
            *self.0 = prev;
        }
    }
}

impl<'t, T: Into<D>, D> SaveFrom<T> for RestoreOnReset<'t, D> {
    fn save_from(&mut self, value: T) {
        *self.0 = value.into()
    }
}

impl<'t, E, D> IntoResettable for SaveScalar<E, &'t mut D> {
    type Resettable = SaveScalar<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        SaveScalar(RestoreOnReset(self.0, None), PhantomData)
    }
}

impl<'t, E, D> Resettable for SaveScalar<E, RestoreOnReset<'t, D>> {
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {
        self.0.mark()
    }

    fn reset(&mut self, to: Self::Mark) {
        self.0.reset(to);
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct SaveRepeated<E, D>(D, PhantomData<E>);

impl<E, D> SaveRepeated<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E: Encoding, D> ScanTypes for SaveRepeated<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'t, E: Encoding, D: DerefMut<Target: Extend<E::Repr>>> OnScanField for SaveRepeated<E, D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
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

impl<'t, E, D> IntoResettable for SaveRepeated<E, &'t mut Vec<D>> {
    type Resettable = SaveRepeated<E, RestoreLenOnReset<'t, Vec<D>>>;
    fn into_resettable(self) -> Self::Resettable {
        SaveRepeated(RestoreLenOnReset(self.0), PhantomData)
    }
}

pub struct RestoreLenOnReset<'t, T>(&'t mut T);

impl<'t, T> Resettable for RestoreLenOnReset<'t, Vec<T>> {
    type Mark = usize;
    fn mark(&mut self) -> Self::Mark {
        self.0.len()
    }
    fn reset(&mut self, to: Self::Mark) {
        self.0.truncate(to);
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

impl<D: Resettable, E> Resettable for SaveRepeated<E, D> {
    type Mark = D::Mark;
    fn mark(&mut self) -> Self::Mark {
        self.0.mark()
    }
    fn reset(&mut self, to: Self::Mark) {
        self.0.reset(to);
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct SaveBytes<E: ?Sized, D>(D, PhantomData<E>);

impl<E: ?Sized, D> SaveBytes<E, D> {
    pub fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E: ?Sized, D> ScanTypes for SaveBytes<E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'t, D: for<'a> From<&'a [u8]>> OnScanField for SaveBytes<[u8], &'t mut D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, _value: ScalarField) -> Result<Option<Infallible>, StopScan> {
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
        *self.0 = bytes.as_ref().into();
        Ok(None)
    }
}

impl<'t, D: for<'a> From<&'a str>> OnScanField for SaveBytes<str, &'t mut D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, _value: ScalarField) -> Result<Option<Infallible>, StopScan> {
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
        *self.0 = bytes.into();
        Ok(None)
    }
}

impl<D: Resettable, E> Resettable for SaveBytes<E, D> {
    type Mark = D::Mark;
    fn mark(&mut self) -> Self::Mark {
        self.0.mark()
    }
    fn reset(&mut self, to: Self::Mark) {
        self.0.reset(to);
    }
}
