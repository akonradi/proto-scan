use std::convert::Infallible;

pub use crate::wire::{FieldNumber, GroupOp, I32, I64, ScalarField, Varint};
use crate::wire::{LengthDelimited, ParseEvent, ParseEventReader};

pub mod encoding;
pub mod field;

/// A message that can be scanned.
pub trait ScanMessage {
    /// The scanner for the message.
    type Scanner;

    /// Creates a new scanner.
    fn scanner() -> Self::Scanner;
}

pub trait ScanTypes {
    /// Output on processing a scan event.
    type ScanEvent;
    /// Result of collecting a sequence of parse events.
    type ScanOutput;
}

/// A scan in progress.
pub trait Scan: ScanTypes + IntoIterator<Item = Result<Self::ScanEvent, StopScan>> {
    /// Performs a complete scan, returning the result.
    fn read_all(self) -> Result<Self::ScanOutput, StopScan>;
}

/// Callbacks for parse inputs encountered during a scan.
pub trait ScanCallbacks: ScanTypes<ScanOutput: FromIterator<Self::ScanEvent>> {
    /// Called when a scalar field is parsed.
    fn on_scalar(
        &mut self,
        field: FieldNumber,
        value: ScalarField,
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

/// A [`Scan`] implementation that takes events from a [`ParseEventReader`] and
/// applies them to a [`ScanCallbacks`].
pub struct ScanWith<P, S>(P, S);

impl<P, S> ScanWith<P, S> {
    pub fn new(input: P, scanner: S) -> Self {
        Self(input, scanner)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> ScanTypes for ScanWith<P, S> {
    type ScanEvent = S::ScanEvent;
    type ScanOutput = S::ScanOutput;
}

/// [`IntoIterator::IntoIter`] type for [`ScanWith`].
/// 
/// Implements [`Iterator`] by applying events from a [`ParseEventReader`] to a
/// [`ScanCallbacks`] and yielding the resulting [`ScanCallbacks::ScanEvent`] or
/// an error.
pub struct IntoIter<P, S>(P, S);

impl<P: ParseEventReader, S: ScanCallbacks> IntoIterator for ScanWith<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    type IntoIter = IntoIter<P, S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0, self.1)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> Iterator for IntoIter<P, S> {
    type Item = Result<S::ScanEvent, StopScan>;
    fn next(&mut self) -> Option<Result<S::ScanEvent, StopScan>> {
        let Self(parse, fields) = self;
        let (field_number, event) = match parse.next() {
            Some(Err(_)) => return Some(Err(StopScan)),
            None => return None,
            Some(Ok(event)) => event,
        };

        let output = match event {
            ParseEvent::Scalar(scalar_field) => fields.on_scalar(field_number, scalar_field),
            ParseEvent::Group(group_op) => fields.on_group(field_number, group_op),
            ParseEvent::LengthDelimited(l) => fields.on_length_delimited(field_number, l),
        };
        Some(output)
    }
}

impl<P: ParseEventReader, S: ScanCallbacks> Scan for ScanWith<P, S> {
    fn read_all(self) -> Result<Self::ScanOutput, StopScan> {
        self.into_iter().collect()
    }
}

/// Sentinel type indicating that a scan completed unsuccessfully.
/// 
/// TODO: make this an enum that provides some detail about why the scan was
/// unsuccessful.
#[derive(Debug)]
pub struct StopScan;

impl From<Infallible> for StopScan {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
