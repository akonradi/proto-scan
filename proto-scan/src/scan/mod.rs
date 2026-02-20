use std::convert::Infallible;

pub use crate::wire::{FieldNumber, GroupOp, I32, I64, ScalarField, Varint};
use crate::wire::{LengthDelimited, ParseEvent, ParseEventReader};

pub mod encoding;
pub mod field;

pub trait ScanMessage {
    type Scanner;

    fn scanner() -> Self::Scanner;
}

pub trait Scan: IntoIterator<Item = Result<Self::Event, StopScan>> {
    type Event;
    type Output;

    fn read_all(self) -> Result<Self::Output, StopScan>;
}

pub trait ScanCallbacks {
    type ScanEvent;
    type ScanOutput: FromIterator<Self::ScanEvent>;

    fn on_scalar(
        &mut self,
        field: FieldNumber,
        value: ScalarField,
    ) -> Result<Self::ScanEvent, StopScan>;

    fn on_group(&mut self, field: FieldNumber, op: GroupOp) -> Result<Self::ScanEvent, StopScan>;

    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl LengthDelimited,
    ) -> Result<Self::ScanEvent, StopScan>;
}

pub struct ScanWith<P, S>(P, S);

impl<P, S> ScanWith<P, S> {
    pub fn new(input: P, scanner: S) -> Self {
        Self(input, scanner)
    }
}

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
    type Event = S::ScanEvent;
    type Output = S::ScanOutput;

    fn read_all(self) -> Result<Self::Output, StopScan> {
        self.into_iter().collect()
    }
}

#[derive(Debug)]
pub struct StopScan;

impl From<Infallible> for StopScan {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
