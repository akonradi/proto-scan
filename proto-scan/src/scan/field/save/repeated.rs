#![doc(hidden)]

#[cfg(feature = "std")]
use crate::scan::field::save::DecodeFromBytes;
#[cfg(feature = "std")]
use crate::scan::field::save::bytes::SaveBytesScanner;

#[cfg(feature = "std")]
use {
    crate::read::ReadTypes,
    crate::scan::encoding::Encoding,
    crate::scan::field::OnScanField,
    crate::scan::field::{Message, RepeatStrategy, RepeatStrategyScanner},
    crate::scan::{IntoResettableScanner, IntoScanOutput, ResettableScanner, ScanError},
    crate::scan::{IntoScanner, MessageScanner, ScanCallbacks},
    crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType, WrongWireType},
    core::convert::Infallible,
};

/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
#[cfg(feature = "std")]
pub struct SaveRepeated<E: Encoding>(Vec<E::Repr>);

#[cfg(feature = "std")]
impl<E: Encoding> SaveRepeated<E> {
    pub(super) fn new() -> Self {
        Self(Vec::new())
    }
}

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
impl<E: Encoding> IntoResettableScanner for SaveRepeated<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

#[cfg(feature = "std")]
impl<E: Encoding> ResettableScanner for SaveRepeated<E> {
    fn reset(&mut self) {
        self.0.clear()
    }
}

/// [`RepeatStrategy`] that clones a message scanner and saves its output in a [`Vec`].
pub struct SaveCloned;

/// Implementation of [`RepeatStrategyScanner`] for [`SaveCloned`].
///
/// On encountering a new embedded message, this clones the scanner provided to
/// [`RepeatStrategyScanner::on_message`], uses it to scan the message input,
/// then saves the output into a [`Vec`]. The saved contents are produced as
/// this type's [`IntoScanOutput::ScanOutput`].
#[cfg(feature = "std")]
pub struct RepeatedSave<S: IntoScanOutput>(Vec<S::ScanOutput>);

#[cfg(feature = "std")]
impl<M: MessageScanner + IntoScanner<M::Message>> RepeatStrategy<M> for SaveCloned {
    type Impl<R: ReadTypes> = RepeatedSave<M::Scanner<R>>;
    fn into_impl<R: ReadTypes>(self) -> Self::Impl<R> {
        RepeatedSave(Vec::new())
    }
}

#[cfg(feature = "std")]
impl<R: ReadTypes, S: ScanCallbacks<R> + IntoScanOutput + Clone> RepeatStrategyScanner<R, S> for RepeatedSave<S> {
    fn on_message(
        &mut self,
        scanner: &S,
        input: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        let mut scanner = Message::new(scanner.clone());
        let _event = scanner.on_length_delimited(input)?;
        self.0.push(scanner.into_scan_output());
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<S: IntoScanOutput> IntoScanOutput for RepeatedSave<S> {
    type ScanOutput = Vec<S::ScanOutput>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
#[cfg(feature = "std")]
pub struct SaveRepeatedBytes<E: DecodeFromBytes + ?Sized, R: ReadTypes>(Vec<E::Decoded<R>>);

#[cfg(feature = "std")]
impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> SaveRepeatedBytes<E, R> {
    pub(super) fn new() -> Self {
        Self(Vec::new())
    }
}

#[cfg(feature = "std")]
impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> IntoScanOutput for SaveRepeatedBytes<E, R> {
    type ScanOutput = Vec<E::Decoded<R>>;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

#[cfg(feature = "std")]
impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> OnScanField<R> for SaveRepeatedBytes<E, R> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let mut scanner = SaveBytesScanner::<E, R>::new();
        let _event = scanner.on_length_delimited(delimited)?;
        let value = scanner.into_scan_output();
        self.0.push(value);
        Ok(None)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }
}

#[cfg(feature = "std")]
impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> IntoResettableScanner for SaveRepeatedBytes<E, R> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

#[cfg(feature = "std")]
impl<E: DecodeFromBytes + ?Sized, R: ReadTypes> ResettableScanner for SaveRepeatedBytes<E, R> {
    fn reset(&mut self) {
        self.0.clear()
    }
}
