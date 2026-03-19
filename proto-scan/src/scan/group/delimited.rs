#![doc(hidden)]

use crate::read::{ReadError, ReadTypes};
use crate::scan::delimited::ScanDelimited;
use crate::scan::group::GroupStack;
use crate::scan::{ParseEventReaderScanError, ScanCallbacks, ScanError};
use crate::wire::{FieldNumber, GroupOp, ParseEvent, ParseEventReader};

/// An accessor for the contents of a proto2 group.
pub trait GroupDelimited {
    type ReadTypes: ReadTypes;

    /// Scans the contents of the group through the provided [`ScanCallbacks`].
    ///
    /// Passes the contents of the group to the provided scanner until the end
    /// tag for the group is read (the end tag is not passed to the scanner).
    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>>;
}

pub(crate) struct GroupDelimitedImpl<'t, P, G> {
    pub(crate) parse: &'t mut P,
    pub(crate) group_stack: &'t mut G,
    pub(crate) open_count: usize,
}

impl<'t, P: ParseEventReader, G: GroupStack> GroupDelimitedImpl<'t, P, G> {
    pub(in crate::scan) fn new(parse: &'t mut P, group_stack: &'t mut G) -> Self {
        Self {
            parse,
            group_stack,
            open_count: 0,
        }
    }

    pub(in crate::scan) fn scan_through<S: ScanCallbacks<P::ReadTypes>>(
        mut self,
        scanner: &mut S,
        field_number: FieldNumber,
    ) -> Result<S::ScanEvent, ScanError<<P::ReadTypes as ReadError>::Error>> {
        self.scan_through_mut(scanner, field_number)
    }

    /// Non-public version that doesn't consume `self`.
    ///
    /// External code should use [Self::scan_through].
    fn scan_through_mut<S: ScanCallbacks<P::ReadTypes>>(
        &mut self,
        scanner: &mut S,
        field_number: FieldNumber,
    ) -> Result<S::ScanEvent, ScanError<<P::ReadTypes as ReadError>::Error>> {
        self.group_stack.push(field_number.into())?;
        self.open_count += 1;
        let event = scanner.on_group(field_number, &mut *self)?;
        self.read_to_end()?;
        Ok(event)
    }

    fn read_to_end(&mut self) -> Result<(), ParseEventReaderScanError<P>> {
        let Self {
            parse,
            group_stack,
            open_count,
        } = self;

        while *open_count != 0 {
            let Some((field_number, event)) = parse.next().transpose()? else {
                return Err(ScanError::GroupMismatch);
            };

            match event {
                ParseEvent::Group(GroupOp::Start) => {
                    group_stack.push(field_number.into())?;
                    *open_count += 1;
                }
                ParseEvent::Group(GroupOp::End) => {
                    *open_count = open_count.checked_sub(1).ok_or(ScanError::GroupMismatch)?;
                    group_stack.pop(field_number.into())?;
                }
                ParseEvent::Numeric(_) | ParseEvent::LengthDelimited(_) => {}
            }
        }
        Ok(())
    }
}

impl<'t, P: ParseEventReader, G: GroupStack> GroupDelimited for &mut GroupDelimitedImpl<'t, P, G> {
    type ReadTypes = P::ReadTypes;

    fn scan_with<S: ScanCallbacks<Self::ReadTypes>>(
        self,
        mut scanner: S,
    ) -> Result<(), ScanError<<Self::ReadTypes as ReadError>::Error>> {
        while self.open_count != 0 {
            let (field_number, event) = self
                .parse
                .next()
                .transpose()?
                .ok_or(ScanError::GroupMismatch)?;
            match event {
                ParseEvent::Group(GroupOp::End) => {
                    self.open_count = self
                        .open_count
                        .checked_sub(1)
                        .ok_or(ScanError::GroupMismatch)?;
                    self.group_stack.pop(field_number.into())?;
                }
                ParseEvent::Group(GroupOp::Start) => {
                    drop(event);
                    self.scan_through_mut(&mut scanner, field_number)?;
                    break;
                }
                ParseEvent::Numeric(numeric_field) => {
                    scanner.on_numeric(field_number, numeric_field)?;
                }
                ParseEvent::LengthDelimited(delimited) => {
                    scanner.on_length_delimited(
                        field_number,
                        ScanDelimited::new(delimited, &mut *self.group_stack),
                    )?;
                }
            }
        }
        Ok(())
    }
}
