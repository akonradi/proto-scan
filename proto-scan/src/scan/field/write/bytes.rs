#![doc(hidden)]

use core::convert::Infallible;
use core::marker::PhantomData;
use core::ops::DerefMut;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::field::save::DecodeFromBytes;
#[cfg(feature = "std")]
use crate::scan::field::write::RestoreLenOnReset;
use crate::scan::field::write::RestoreOnReset;
use crate::scan::save_from::SaveFrom;
use crate::scan::{
    GroupOp, IntoResettableScanner, IntoScanOutput, NumericField, ResettableScanner, ScanError,
    ScanLengthDelimited,
};
use crate::wire::WrongWireType;

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteBytes<E: ?Sized, D>(D, PhantomData<E>);

impl<E: ?Sized, D> WriteBytes<E, D> {
    pub(super) fn new(to: D) -> Self {
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
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
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
        WriteBytes(RestoreOnReset::new(self.0), PhantomData)
    }
}

/// [`OnScanField`] that writes the decoded values to the provided location.
pub struct WriteRepeatedBytes<E: ?Sized, D>(D, PhantomData<E>);

impl<E: ?Sized, D> WriteRepeatedBytes<E, D> {
    pub(super) fn new(to: D) -> Self {
        Self(to, PhantomData)
    }
}

impl<E: ?Sized, D> IntoScanOutput for WriteRepeatedBytes<E, D> {
    type ScanOutput = ();

    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<B: DecodeFromBytes + ?Sized, D: DerefMut<Target: Extend<B::Decoded<R>>>, R: ReadTypes>
    OnScanField<R> for WriteRepeatedBytes<B, D>
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
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Infallible>, ScanError<R::Error>> {
        let bytes = delimited.into_bytes()?;
        let decoded = B::decode(bytes).map_err(|_| ScanError::Utf8)?;
        self.0.extend([decoded]);
        Ok(None)
    }
}

#[cfg(feature = "std")]
impl<'t, E: ?Sized, D> IntoResettableScanner for WriteRepeatedBytes<E, &'t mut Vec<D>> {
    type Resettable = WriteRepeatedBytes<E, RestoreLenOnReset<'t, Vec<D>>>;
    fn into_resettable(self) -> Self::Resettable {
        WriteRepeatedBytes(RestoreLenOnReset::new(self.0), PhantomData)
    }
}

#[cfg(feature = "std")]
impl<'t, E: ?Sized, D> ResettableScanner for WriteRepeatedBytes<E, RestoreLenOnReset<'t, Vec<D>>> {
    fn reset(&mut self) {
        self.0.reset();
    }
}
