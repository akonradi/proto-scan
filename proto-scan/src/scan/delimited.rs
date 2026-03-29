use crate::read::{Read, ReadTypes};
use crate::scan::{GroupStack, IntoScanOutput, ScanCallbacks, ScanError};
use crate::wire::{DelimitedTypes, LengthDelimited, ParseEventReader};

/// Allows scanning the contents of a delimited field.
pub trait ScanDelimited: DelimitedTypes {
    /// Scans the contents of the delimited field.
    ///
    /// The provided [`ScanCallbacks`] implementation will have its methods
    /// invoked for each scan event.
    fn scan_with<S: ScanCallbacks<Self::ReadTypes> + IntoScanOutput>(
        self,
        scanner: S,
    ) -> Result<S::ScanOutput, ScanError<<Self::ReadTypes as ReadTypes>::Error>>;
}

/// Functionally an alias for [`LengthDelimited`] plus [`ScanDelimited`].
///
/// Blanket-implemented for all compatible types.
pub trait ScanLengthDelimited: LengthDelimited + ScanDelimited {}
impl<L: LengthDelimited + ScanDelimited> ScanLengthDelimited for L {}

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
        crate::read::ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        self.length_delimited.into_bytes()
    }

    fn into_reader(self) -> impl Read<ReadTypes = Self::ReadTypes> {
        self.length_delimited.into_reader()
    }

    fn into_events(self) -> impl ParseEventReader<ReadTypes = Self::ReadTypes> {
        self.length_delimited.into_events()
    }

    fn is_empty(&self) -> bool {
        self.length_delimited.is_empty()
    }
}

impl<L: LengthDelimited, G: GroupStack> ScanDelimited for ScanDelimitedImpl<'_, L, G> {
    fn scan_with<S: ScanCallbacks<Self::ReadTypes> + IntoScanOutput>(
        self,
        mut scanner: S,
    ) -> Result<S::ScanOutput, ScanError<<Self::ReadTypes as ReadTypes>::Error>> {
        let Self {
            group_stack,
            length_delimited,
        } = self;
        let mut parse = length_delimited.into_events();

        while let Some(r) = super::next_event(&mut parse, &mut scanner, group_stack) {
            r?;
        }
        Ok(scanner.into_scan_output())
    }
}
