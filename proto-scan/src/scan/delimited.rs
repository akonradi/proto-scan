use crate::read::{ReadError, ReadTypes};
use crate::scan::{GroupStack, ScanCallbacks, ScanError};
use crate::wire::{DelimitedTypes, LengthDelimited, ParseEventReader};

pub trait ScanDelimited: DelimitedTypes {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>>;
}

pub trait ScanLengthDelimited: LengthDelimited + ScanDelimited {}

pub(super) struct ScanDelimitedImpl<'g, L, G> {
    length_delimited: L,
    group_stack: &'g mut G,
}

impl<'g, L, G> ScanDelimitedImpl<'g, L, G> {
    pub(super) fn new(length_delimited: L, group_stack: &'g mut G) -> Self {
        Self {
            length_delimited,
            group_stack,
        }
    }
}

impl<L: LengthDelimited, G> DelimitedTypes for ScanDelimitedImpl<'_, L, G> {
    type ReadTypes = L::ReadTypes;
}

impl<L: LengthDelimited, G> LengthDelimited for ScanDelimitedImpl<'_, L, G> {
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

impl<L: LengthDelimited, G: GroupStack> ScanDelimited for ScanDelimitedImpl<'_, L, G> {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        mut scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>> {
        let Self {
            group_stack,
            length_delimited,
        } = self;
        let mut parse = length_delimited.into_events();

        while let Some(r) = super::next_event(&mut parse, &mut scanner, group_stack) {
            r?;
        }
        Ok(())
    }
}

impl<L: LengthDelimited, G: GroupStack> ScanLengthDelimited for ScanDelimitedImpl<'_, L, G> {}
