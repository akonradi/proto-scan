#![doc(hidden)]

use core::convert::Infallible;
use core::marker::PhantomData;
use core::ops::DerefMut;

use crate::read::ReadTypes;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
#[cfg(feature = "std")]
use crate::scan::field::write::RestoreLenOnReset;
use crate::scan::field::write::RestoreOnReset;
use crate::scan::save_from::SaveFrom;
use crate::scan::{
    GroupDelimited, IntoResettableScanner, IntoScanOutput, NumericField, ResettableScanner,
    ScanError, ScanLengthDelimited,
};
use crate::wire::{NumericWireType, WrongWireType};

/// [`OnScanField`] that writes the decoded value to the provided location.
pub struct WriteNumeric<E, D>(D, PhantomData<E>);

impl<E, D> WriteNumeric<E, D> {
    pub(super) fn new(destination: D) -> Self {
        Self(destination, PhantomData)
    }
}

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

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl ScanLengthDelimited,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

impl<E: Encoding, D> IntoScanOutput for WriteNumeric<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<'t, E, D> IntoResettableScanner for WriteNumeric<E, &'t mut D> {
    type Resettable = WriteNumeric<E, RestoreOnReset<'t, D>>;

    fn into_resettable(self) -> Self::Resettable {
        WriteNumeric(RestoreOnReset::new(self.0), PhantomData)
    }
}

impl<'t, E, D> ResettableScanner for WriteNumeric<E, RestoreOnReset<'t, D>> {
    fn reset(&mut self) {
        self.0.reset();
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteRepeatedNumeric<E, D>(D, PhantomData<E>);

impl<E, D> WriteRepeatedNumeric<E, D> {
    pub(super) fn new(destination: D) -> Self {
        Self(destination, PhantomData)
    }
}

impl<E: Encoding, R: ReadTypes, D: DerefMut<Target: Extend<E::Repr>>> OnScanField<R>
    for WriteRepeatedNumeric<E, D>
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

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
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

impl<E: Encoding, D> IntoScanOutput for WriteRepeatedNumeric<E, D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

#[cfg(feature = "std")]
impl<'t, E, D> IntoResettableScanner for WriteRepeatedNumeric<E, &'t mut Vec<D>> {
    type Resettable = WriteRepeatedNumeric<E, RestoreLenOnReset<'t, Vec<D>>>;
    fn into_resettable(self) -> Self::Resettable {
        WriteRepeatedNumeric(RestoreLenOnReset::new(self.0), PhantomData)
    }
}

impl<D: ResettableScanner, E> ResettableScanner for WriteRepeatedNumeric<E, D> {
    fn reset(&mut self) {
        self.0.reset();
    }
}
