use core::num::NonZeroU32;

use assert_matches::debug_assert_matches;
use either::Either;

use crate::DecodeError;
use crate::read::{Read, ReadError, ReadTypes};
use crate::wire::parse::{DoBeforeNext, EventReader, LimitReader, NumericIter};
use crate::wire::{LengthDelimited, NumericWireType, ParseEventReader};

pub(super) struct LengthDelimitedImpl<'a, R> {
    pub(super) reader: LimitReader<&'a mut R>,
    pub(super) write_back_to: &'a mut DoBeforeNext,
}

impl<R: Read> Read for LengthDelimitedImpl<'_, R> {
    type ReadTypes = R::ReadTypes;

    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<<Self::ReadTypes as ReadTypes>::Buffer, <Self::ReadTypes as ReadError>::Error> {
        match self.reader.read(bytes) {
            Ok(b) => Ok(b),
            Err(e) => {
                *self.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        }
    }

    fn skip(&mut self, bytes: u32) -> Result<u32, <Self::ReadTypes as ReadError>::Error> {
        match self.reader.skip(bytes) {
            Ok(b) => Ok(b),
            Err(e) => {
                *self.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        }
    }
}

impl<'a, R: Read> LengthDelimited for LengthDelimitedImpl<'a, R> {
    type ReadTypes = R::ReadTypes;
    type PackedIter<W: NumericWireType> = NumericIter<'a, R, W>;

    fn len(&self) -> u32 {
        self.reader.remaining()
    }

    fn into_bytes(
        mut self,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        DecodeError<<Self::ReadTypes as ReadError>::Error>,
    > {
        let remaining = self.reader.remaining();
        let bytes = self.reader.read(remaining).map_err(DecodeError::Read)?;
        if bytes.as_ref().len() != remaining as usize {
            *self.write_back_to = DoBeforeNext::Error;
            return Err(DecodeError::UnexpectedEnd);
        }
        Ok(bytes)
    }

    fn into_packed<W: NumericWireType>(self) -> Self::PackedIter<W> {
        NumericIter::<_, W>::new(self)
    }

    fn into_events(self) -> impl ParseEventReader<ReadTypes = Self::ReadTypes> {
        EventReader {
            inner: Either::Right(self),
            do_before: DoBeforeNext::DoNothing,
        }
    }
}

impl<'a, R> Drop for LengthDelimitedImpl<'a, R> {
    fn drop(&mut self) {
        let Self {
            reader,
            write_back_to,
        } = self;
        if let Some(remaining) = NonZeroU32::new(reader.remaining()) {
            debug_assert_matches!(write_back_to, DoBeforeNext::DoNothing | DoBeforeNext::Error);
            match write_back_to {
                DoBeforeNext::DoNothing | DoBeforeNext::Skip(_) => {
                    **write_back_to = DoBeforeNext::Skip(remaining)
                }
                DoBeforeNext::Error => {}
            }
        }
    }
}
