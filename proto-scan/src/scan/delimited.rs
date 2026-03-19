use crate::read::{ReadError, ReadTypes};
use crate::scan::{Scan, ScanCallbacks, ScanError};
use crate::wire::{LengthDelimited, ParseEventReader};

pub trait ScanLengthDelimited: LengthDelimited {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>>;
}

pub(super) struct ScanDelimited<L> {
    length_delimited: L,
}

impl<L> ScanDelimited<L> {
    pub(super) fn new(length_delimited: L) -> Self {
        Self { length_delimited }
    }
}

impl<L: LengthDelimited> LengthDelimited for ScanDelimited<L> {
    type ReadTypes = L::ReadTypes;

    fn len(&self) -> u32 {
        self.length_delimited.len()
    }

    type PackedIter<W: crate::wire::NumericWireType> = L::PackedIter<W>;

    fn into_packed<W: crate::wire::NumericWireType>(self) -> Self::PackedIter<W> {
        self.length_delimited.into_packed()
    }

    fn into_bytes(
        self,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        crate::DecodeError<<Self::ReadTypes as ReadError>::Error>,
    > {
        self.length_delimited.into_bytes()
    }

    fn into_events(self) -> impl ParseEventReader<ReadTypes = Self::ReadTypes> {
        self.length_delimited.into_events()
    }
}

impl<L: LengthDelimited> ScanLengthDelimited for ScanDelimited<L> {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>> {
        for r in Scan::new(self.length_delimited.into_events(), scanner) {
            r?;
        }
        Ok(())
    }
}
