use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::wire::{FieldNumber, GroupOp, I32, I64, NumericField, Varint};
use crate::wire::{LengthDelimited, ParseEvent, ParseEventReader};

mod builder;
pub use builder::ScannerBuilder;
pub mod encoding;
pub mod field;
mod resettable;
pub use resettable::{IntoResettable, Resettable};
mod repeated;
pub use repeated::{
    Fold, RepeatStrategy, RepeatStrategyScanner, Repeated, SaveCloned, ScanRepeated, WriteCloned,
};
mod save_from;
pub use save_from::SaveFrom;

/// A message that can be scanned.
pub trait ScanMessage {
    /// The scanner for the message.
    type ScannerBuilder: ScannerBuilder<Self>;

    /// Creates a new scanner builder
    fn scanner() -> Self::ScannerBuilder;
}

/// A scanner for a protobuf message type.
///
/// This functions as a marker trait for scanner types that specifically scan
/// for messages (not fields or oneofs).
pub trait MessageScanner {
    /// The message type that this scanner can read.
    type Message: ScanMessage;
}

pub trait IntoScanner<T: ?Sized> {
    type Scanner<R: ReadTypes>: IntoScanOutput;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R>;
}

pub trait IntoScanOutput {
    type ScanOutput;
    fn into_scan_output(self) -> Self::ScanOutput;
}

/// Callbacks for parse inputs encountered during a scan.
pub trait ScanCallbacks<R: ReadTypes, F = FieldNumber>: IntoScanOutput {
    type ScanEvent;

    /// Called when a numeric field is parsed.
    fn on_numeric(&mut self, field: F, value: NumericField) -> Result<Self::ScanEvent, StopScan>;

    /// Called when a SGROUP or EGROUP tag is read.
    fn on_group(&mut self, field: F, op: GroupOp) -> Result<Self::ScanEvent, StopScan>;

    /// Called when a length-delimited field tag is encountered.
    fn on_length_delimited(
        &mut self,
        field: F,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, StopScan>;
}

/// A oneof grouping in a message that can be scanned for.
pub trait ScannableOneOf {
    /// A discriminant for the oneof variants.
    ///
    /// This should be an enum with one variant for each proto field in the oneof.
    type FieldNumber;
}

/// A scan in progress.
pub struct Scan<P, S>(P, S);

impl<P, S> Scan<P, S> {
    pub fn new(input: P, scanner: S) -> Self {
        Self(input, scanner)
    }
}

/// [`IntoIterator::IntoIter`] type for [`Scan`].
///
/// Implements [`Iterator`] by applying events from a [`ParseEventReader`] to a
/// [`ScanCallbacks`] and yielding the resulting [`ScanTypes::ScanEvent`] or
/// an error.
pub struct IntoIter<P, S>(P, S);

impl<P: ParseEventReader, S: ScanCallbacks<P::ReadTypes>> IntoIterator for Scan<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    type IntoIter = IntoIter<P, S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0, self.1)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks<P::ReadTypes>> Iterator for IntoIter<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    fn next(&mut self) -> Option<Result<S::ScanEvent, StopScan>> {
        next_event(&mut self.0, &mut self.1)
    }
}

pub(crate) fn next_event<P: ParseEventReader, S: ScanCallbacks<P::ReadTypes>>(
    parse: &mut P,
    fields: &mut S,
) -> Option<Result<S::ScanEvent, StopScan>> {
    let (field_number, event) = match parse.next() {
        Some(Err(_)) => return Some(Err(StopScan)),
        None => return None,
        Some(Ok(event)) => event,
    };

    let output = match event {
        ParseEvent::Numeric(numeric_field) => fields.on_numeric(field_number, numeric_field),
        ParseEvent::Group(group_op) => fields.on_group(field_number, group_op),
        ParseEvent::LengthDelimited(l) => fields.on_length_delimited(field_number, l),
    };
    Some(output)
}

impl<P: ParseEventReader, S: ScanCallbacks<P::ReadTypes>> Scan<P, S> {
    pub fn read_all(self) -> Result<S::ScanOutput, StopScan> {
        let mut it = self.into_iter();
        for r in it.by_ref() {
            let _ = r?;
        }
        Ok(it.1.into_scan_output())
    }
}

/// Sentinel type indicating that a scan completed unsuccessfully.
///
/// TODO: make this an enum that provides some detail about why the scan was
/// unsuccessful.
#[derive(Debug, PartialEq)]
pub struct StopScan;

impl From<Infallible> for StopScan {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
