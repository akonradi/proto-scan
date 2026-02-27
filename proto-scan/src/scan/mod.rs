use std::convert::Infallible;

use crate::read::Read;
use crate::wire::{FieldNumber, GroupOp, I32, I64, NumericField, Varint};
use crate::wire::{LengthDelimited, ParseEvent, ParseEventReader};

pub mod encoding;
pub mod field;
mod resettable;
pub use resettable::{IntoResettable, Resettable};

/// A message that can be scanned.
pub trait ScanMessage {
    /// The scanner for the message.
    type ScannerBuilder: ScannerBuilder<Message = Self>;

    /// Creates a new scanner builder
    fn scanner() -> Self::ScannerBuilder;
}

pub trait ScanTypes {
    /// Output on processing a scan event.
    type ScanEvent;
    /// Result of collecting a sequence of parse events.
    type ScanOutput;
}

/// A builder type for a [`Scan`] over a byte stream.
pub trait ScannerBuilder: ScanTypes + Sized {
    type Message;

    /// Starts a scan over the provided input.
    ///
    /// Consumes `self` and produces a [`Scan`] over the input stream.
    fn scan_events<P: ParseEventReader>(self, read: P) -> Scan<P, Self::Scanner>
    where
        Self: IntoScanner,
    {
        Scan::new(read, self.into_scanner())
    }

    /// Starts a scan over the provided input.
    ///
    /// Consumes `self` and produces a [`Scan`] over the input stream.
    fn scan<'r>(self, read: impl Read + 'r) -> Scan<impl ParseEventReader + 'r, Self::Scanner>
    where
        Self: IntoScanner,
    {
        Scan::new(crate::wire::parse(read), self.into_scanner())
    }
}

pub trait IntoScanner {
    type Scanner;
    fn into_scanner(self) -> Self::Scanner;
}

/// Callbacks for parse inputs encountered during a scan.
pub trait ScanCallbacks: ScanTypes {
    /// Called when a numeric field is parsed.
    fn on_numeric(
        &mut self,
        field: FieldNumber,
        value: NumericField,
    ) -> Result<Self::ScanEvent, StopScan>;

    /// Called when a SGROUP or EGROUP tag is read.
    fn on_group(&mut self, field: FieldNumber, op: GroupOp) -> Result<Self::ScanEvent, StopScan>;

    /// Called when a length-delimited field tag is encountered.
    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl LengthDelimited,
    ) -> Result<Self::ScanEvent, StopScan>;
}

/// A scan in progress.
pub struct Scan<P, S>(P, S);

impl<P, S> Scan<P, S> {
    pub fn new(input: P, scanner: S) -> Self {
        Self(input, scanner)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> ScanTypes for Scan<P, S> {
    type ScanEvent = S::ScanEvent;
    type ScanOutput = S::ScanOutput;
}

/// [`IntoIterator::IntoIter`] type for [`Scan`].
///
/// Implements [`Iterator`] by applying events from a [`ParseEventReader`] to a
/// [`ScanCallbacks`] and yielding the resulting [`ScanTypes::ScanEvent`] or
/// an error.
pub struct IntoIter<P, S>(P, S);

impl<P: ParseEventReader, S: ScanCallbacks> IntoIterator for Scan<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    type IntoIter = IntoIter<P, S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0, self.1)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> Iterator for IntoIter<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    fn next(&mut self) -> Option<Result<S::ScanEvent, StopScan>> {
        next_event(&mut self.0, &mut self.1)
    }
}

pub(crate) fn next_event<P: ParseEventReader, S: ScanCallbacks>(
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

impl<P: ParseEventReader, S: ScanCallbacks + Into<S::ScanOutput>> Scan<P, S> {
    pub fn read_all(self) -> Result<<Self as ScanTypes>::ScanOutput, StopScan> {
        let mut it = self.into_iter();
        while let Some(r) = it.next() {
            let _ = r?;
        }
        Ok(it.1.into())
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
