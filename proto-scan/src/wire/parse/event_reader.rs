use crate::read::count_reader::CountReader;
use crate::read::{Read, ReadBytesError, ReadTypes};
use crate::wire::parse::{DoBeforeNext, LengthDelimitedImpl, LimitReader};
use crate::wire::{
    FieldNumber, GroupOp, I32, I64, LengthDelimited, NumericField, NumericWireType, ParseEvent,
    ParseEventReader, Tag, Varint, WireType,
};
use crate::{DecodeError, DecodeVarintError};

pub(super) struct EventReader<R> {
    inner: R,
    do_before: DoBeforeNext,
}

impl<R: Read> EventReader<BaseEventReader<R>> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            inner: BaseEventReader(reader),
            do_before: DoBeforeNext::DoNothing,
        }
    }
}

impl<R: EventRead> ParseEventReader for EventReader<R> {
    type ReadTypes = R::ReadTypes;
    #[inline]
    fn next(
        &mut self,
    ) -> Option<
        Result<
            (
                FieldNumber,
                ParseEvent<impl LengthDelimited<ReadTypes = Self::ReadTypes>>,
            ),
            DecodeError<<R::ReadTypes as ReadTypes>::Error>,
        >,
    > {
        let Self {
            inner: reader,
            do_before,
        } = self;

        match core::mem::take(do_before) {
            DoBeforeNext::Skip(to_skip) => {
                if let Err(e) = reader.skip_direct(to_skip.get()) {
                    return Some(Err(e.into()));
                }
            }
            DoBeforeNext::DoNothing => {}
        }
        let mut tag_reader = CountReader::new(&mut *reader);

        let tag = match Tag::read_from(&mut tag_reader) {
            Ok(tag) => tag,
            Err(DecodeError::UnexpectedEnd) if tag_reader.count() == 0 => return None,
            Err(e) => return Some(Err(e)),
        };

        let Tag {
            wire_type,
            field_number,
        } = tag;

        Some(
            match wire_type {
                WireType::Varint => Varint::read_from(&mut *reader)
                    .map(|value| ParseEvent::Numeric(NumericField::Varint(value))),
                WireType::I64 => I64::read_from(reader)
                    .map(|value| ParseEvent::Numeric(NumericField::I64(value))),
                WireType::I32 => I32::read_from(reader)
                    .map(|value| ParseEvent::Numeric(NumericField::I32(value))),
                WireType::Sgroup => Ok(ParseEvent::Group(GroupOp::Start)),
                WireType::Egroup => Ok(ParseEvent::Group(GroupOp::End)),
                WireType::LengthDelimited => {
                    let original_length = match (|| {
                        let length = Varint::read_from(reader)?;
                        u32::try_from(length)
                            .map_err(|_| DecodeError::TooLargeLengthDelimited(length))
                    })() {
                        Err(e) => return Some(Err(e)),
                        Ok(l) => l,
                    };

                    Ok(ParseEvent::LengthDelimited(LengthDelimitedImpl {
                        reader: reader.take_as_reader(original_length),
                        write_back_to: do_before,
                    }))
                }
            }
            .map(|field| (field_number, field)),
        )
    }

    #[cfg(feature = "tail-call")]
    fn read_all<S: super::ParseCallbacks<Self::ReadTypes> + crate::scan::IntoScanOutput>(
        self,
        scanner: S,
    ) -> Result<S::ScanOutput, S::ParseError> {
        super::tail_call::read_all(self.inner, scanner)
    }
}

/// Helper for [`EventReader`].
pub(super) trait EventRead: Read {
    type RealReader: Read<ReadTypes = Self::ReadTypes>;

    /// Takes some number of bytes as a [`LimitReader`].
    /// 
    /// The caller must guarante that after the `LimitReader` is dropped,
    /// [`Self::skip_direct`] is called with the count of bytes that were taken
    /// but that it didn't consume.
    fn take_as_reader(&mut self, bytes: u32) -> LimitReader<&mut Self::RealReader>;

    /// Skips some number of bytes.
    /// 
    /// Skips without adjusting any counters, assuming they were already
    /// adjusted by [`Self::take_as_reader`].
    fn skip_direct(
        &mut self,
        bytes: u32,
    ) -> Result<(), DecodeError<<Self::ReadTypes as ReadTypes>::Error>>;
}

/// Wrapper type that implements [`EventRead`] for a [`Read`].
pub(super) struct BaseEventReader<R>(R);

impl<R: Read> Read for BaseEventReader<R> {
    type ReadTypes = R::ReadTypes;
    #[inline]
    fn read(
        &mut self,
        bytes: u32,
    ) -> Result<
        <Self::ReadTypes as ReadTypes>::Buffer,
        ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>,
    > {
        R::read(&mut self.0, bytes)
    }

    #[inline]
    fn read_varint(
        &mut self,
    ) -> Result<u64, DecodeVarintError<<Self::ReadTypes as ReadTypes>::Error>> {
        R::read_varint(&mut self.0)
    }

    #[inline]
    fn skip(
        &mut self,
        bytes: u32,
    ) -> Result<u32, ReadBytesError<<Self::ReadTypes as ReadTypes>::Error>> {
        R::skip(&mut self.0, bytes)
    }
}

impl<R: Read> EventRead for BaseEventReader<R> {
    type RealReader = R;

    #[inline]
    fn skip_direct(
        &mut self,
        bytes: u32,
    ) -> Result<(), DecodeError<<Self::ReadTypes as ReadTypes>::Error>> {
        let _ = self.0.skip(bytes)?;
        Ok(())
    }

    fn take_as_reader(&mut self, bytes: u32) -> LimitReader<&mut Self::RealReader> {
        LimitReader::new(&mut self.0, bytes)
    }
}
