use std::convert::Infallible;
use std::marker::PhantomData;

use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{GroupOp, ScalarField, ScanTypes, StopScan};
use crate::wire::{LengthDelimited, ScalarWireType};

/// [`OnScanField`] that writes the decoded value to the provided location.
pub struct SaveScalar<'t, E, D>(&'t mut D, PhantomData<E>);

impl<'t, E, D> SaveScalar<'t, E, D> {
    pub fn new(to: &'t mut D) -> Self {
        Self(to, PhantomData)
    }
}

impl<'t, E: Encoding, D: From<E::Repr>> ScanTypes for SaveScalar<'t, E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'t, E: Encoding, D: From<E::Repr>> OnScanField for SaveScalar<'t, E, D> {
    fn into_output(self) -> Self::ScanOutput {}

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Infallible>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
        *self.0 = E::decode(value).map_err(Into::into)?.into();
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

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct SaveRepeated<'t, E: Encoding, D>(&'t mut D, PhantomData<E>);

impl<'t, E: Encoding, D> SaveRepeated<'t, E, D> {
    pub fn new(to: &'t mut D) -> Self {
        Self(to, PhantomData)
    }
}

impl<'t, E: Encoding, D> ScanTypes for SaveRepeated<'t, E, D> {
    type ScanEvent = Infallible;
    type ScanOutput = ();
}

impl<'t, E: Encoding, D: Extend<E::Repr>> OnScanField for SaveRepeated<'t, E, D> {
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
