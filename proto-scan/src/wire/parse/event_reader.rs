use either::Either;

use crate::DecodeError;
use crate::read::count_reader::CountReader;
use crate::read::{Read, ReadTypes};
use crate::wire::parse::{DoBeforeNext, LengthDelimitedImpl, LimitReader};
use crate::wire::{
    FieldNumber, GroupOp, I32, I64, LengthDelimited, NumericField, NumericWireType, ParseEvent,
    ParseEventReader, Tag, Varint, WireType,
};

pub(super) struct EventReader<'a, R> {
    pub(super) inner: Either<R, LengthDelimitedImpl<'a, R>>,
    pub(super) do_before: DoBeforeNext,
}

impl<'a, R: Read> ParseEventReader for EventReader<'a, R> {
    type ReadTypes = R::ReadTypes;
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
        let Self { inner, do_before } = self;

        let reader = inner;

        match core::mem::take(do_before) {
            DoBeforeNext::Skip(to_skip) => {
                if let Err(e) = reader.skip(to_skip.get()) {
                    return Some(Err(e.into()));
                }
            }
            DoBeforeNext::Error => return Some(Err(DecodeError::UnexpectedEnd)),
            DoBeforeNext::DoNothing => {}
        }

        let mut tag_reader = CountReader::new(reader);

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
                WireType::Varint => Varint::read_from(&mut self.inner)
                    .map(|value| ParseEvent::Numeric(NumericField::Varint(value))),
                WireType::I64 => I64::read_from(&mut &mut self.inner)
                    .map(|value| ParseEvent::Numeric(NumericField::I64(value))),
                WireType::I32 => I32::read_from(&mut &mut self.inner)
                    .map(|value| ParseEvent::Numeric(NumericField::I32(value))),
                WireType::Sgroup => Ok(ParseEvent::Group(GroupOp::Start)),
                WireType::Egroup => Ok(ParseEvent::Group(GroupOp::End)),
                WireType::LengthDelimited => {
                    let to_skip = match (|| {
                        let length = Varint::read_from(&mut self.inner)?;
                        u32::try_from(length)
                            .map_err(|_| DecodeError::TooLargeLengthDelimited(length))
                    })() {
                        Err(e) => return Some(Err(e)),
                        Ok(l) => l,
                    };

                    Ok(ParseEvent::LengthDelimited(LengthDelimitedImpl {
                        reader: LimitReader::new(&mut self.inner, to_skip),
                        write_back_to: do_before,
                    }))
                }
            }
            .map(|field| (field_number, field)),
        )
    }
}
