use crate::read::{ReadError, ReadTypes};
use crate::scan::{GroupStack, ScanCallbacks, ScanError};
use crate::wire::{LengthDelimited, ParseEventReader};

pub trait ScanLengthDelimited: LengthDelimited {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>>;
}

pub(super) struct ScanDelimited<'g, L, G> {
    length_delimited: L,
    group_stack: &'g mut G,
}

impl<'g, L, G> ScanDelimited<'g, L, G> {
    pub(super) fn new(length_delimited: L, group_stack: &'g mut G) -> Self {
        Self {
            length_delimited,
            group_stack,
        }
    }
}

impl<L: LengthDelimited, G> LengthDelimited for ScanDelimited<'_, L, G> {
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

impl<L: LengthDelimited, G: GroupStack> ScanLengthDelimited for ScanDelimited<'_, L, G> {
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
