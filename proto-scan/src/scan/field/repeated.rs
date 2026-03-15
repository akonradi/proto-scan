use core::convert::Infallible;
use core::ops::DerefMut;

use crate::read::ReadTypes;
use crate::scan::field::{Message, OnScanField};
use crate::scan::{IntoScanOutput, IntoScanner, MessageScanner, ScanCallbacks, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField, WrongWireType};

/// Marker type for protobuf `repeated`.
pub struct Repeated<T>(pub T);

/// [`RepeatStrategy`] that folds message scanner outputs together.
pub struct Fold<F>(F);

impl<F> Fold<F> {
    pub fn new<T, R>(f: F) -> Self
    where
        F: Fn(&mut T, T) -> R,
    {
        Self(f)
    }
}

/// [`RepeatStrategy`] that clones a message scanner and writes its output somewhere else.
pub struct WriteCloned<D>(pub D);

/// A strategy for handling repeated messages.
pub trait RepeatStrategy<M: MessageScanner> {
    type Impl<R: ReadTypes>: IntoScanOutput;
    fn into_impl<R: ReadTypes>(self) -> Self::Impl<R>;
}

/// The instantiation of a [`RepeatStrategy`] policy.
pub trait RepeatStrategyScanner<R: ReadTypes, S: ScanCallbacks<R>>: IntoScanOutput {
    fn on_message(
        &mut self,
        scanner: &S,
        input: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>>;
}

/// Extension trait for message scanners.
///
/// This adds a `repeat_by` method for transforming a message scanner into a
/// scanner for a repeated message field.
///
/// This trait is blanket-implemented for all [`MessageScanner`]s.
pub trait ScanRepeated: MessageScanner {
    fn repeat_by<R: RepeatStrategy<Self>>(self, strategy: R) -> RepeatedScanner<Self, R>
    where
        Self: Sized,
    {
        RepeatedScanner(self, strategy)
    }
}
impl<M: MessageScanner> ScanRepeated for M {}

/// Implementation of [`RepeatStrategyScanner`].
///
/// This holds a [`MessageScanner`] and a [`RepeatStrategy`] and delegates to
/// them to implement [`OnScanField`] for repeated message fields.
pub struct RepeatedScanner<S, R>(S, R);

impl<S: MessageScanner + IntoScanner<S::Message>, F: RepeatStrategy<S>>
    IntoScanner<Repeated<S::Message>> for RepeatedScanner<S, F>
{
    type Scanner<R: ReadTypes> = RepeatedScanner<S::Scanner<R>, F::Impl<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        RepeatedScanner(self.0.into_scanner(), self.1.into_impl())
    }
}

impl<S, F: IntoScanOutput> IntoScanOutput for RepeatedScanner<S, F> {
    type ScanOutput = F::ScanOutput;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.1.into_scan_output()
    }
}

impl<R: ReadTypes, S: ScanCallbacks<R>, F: RepeatStrategyScanner<R, S>> OnScanField<R>
    for RepeatedScanner<S, F>
{
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
        self.1.on_message(&self.0, delimited).map(|()| None)
    }
}

/// Implementation of [`RepeatStrategyScanner`] for [`Fold`].
///
/// On encountering a new embedded message, this clones the scanner provided to
/// [`RepeatStrategyScanner::on_message`] and uses it to scan the message input.
/// Then, if that was the first instance of the message, it saves the scanner
/// output.  Otherwise it uses the provided closure to fold the new scanner
/// output together with the previous output. The folded output is produced as
/// this type's [`IntoScanOutput::ScanOutput`].
pub struct RepeatedFold<S: IntoScanOutput, F>(Option<S::ScanOutput>, F);

impl<F, S: MessageScanner + IntoScanner<S::Message>> RepeatStrategy<S> for Fold<F> {
    type Impl<R: ReadTypes> = RepeatedFold<S::Scanner<R>, F>;
    fn into_impl<R: ReadTypes>(self) -> Self::Impl<R> {
        RepeatedFold(None, self.0)
    }
}

impl<S: IntoScanOutput, F> IntoScanOutput for RepeatedFold<S, F> {
    type ScanOutput = Option<S::ScanOutput>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<R: ReadTypes, S: ScanCallbacks<R> + Clone, F: Fn(&mut S::ScanOutput, S::ScanOutput)>
    RepeatStrategyScanner<R, S> for RepeatedFold<S, F>
{
    fn on_message(
        &mut self,
        scanner: &S,
        input: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        let mut scanner = Message::new(scanner.clone());
        let _event = scanner.on_length_delimited(input)?;
        if let Some(prev) = self.0.as_mut() {
            self.1(prev, scanner.into_scan_output());
        } else {
            self.0 = Some(scanner.into_scan_output())
        }
        Ok(())
    }
}

pub struct RepeatedWriteCloned<D>(D);

impl<M: MessageScanner, D> RepeatStrategy<M> for WriteCloned<D> {
    type Impl<R: ReadTypes> = RepeatedWriteCloned<D>;

    fn into_impl<R: ReadTypes>(self) -> Self::Impl<R> {
        RepeatedWriteCloned(self.0)
    }
}

impl<D> IntoScanOutput for RepeatedWriteCloned<D> {
    type ScanOutput = ();
    fn into_scan_output(self) -> Self::ScanOutput {}
}

impl<R: ReadTypes, S: ScanCallbacks<R> + Clone, D: DerefMut<Target: Extend<S::ScanOutput>>>
    RepeatStrategyScanner<R, S> for RepeatedWriteCloned<D>
{
    fn on_message(
        &mut self,
        scanner: &S,
        input: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        let mut scanner = Message::new(scanner.clone());
        let _event = scanner.on_length_delimited(input);
        let output = scanner.into_scan_output();
        self.0.extend([output]);
        Ok(())
    }
}
