use either::Either;

use crate::DecodeError;
use crate::visitor::{ScalarField, VisitMessage, Visitor};
use crate::wire::{FieldNumber, I32, I64, LengthDelimited, ScalarWireType, Varint};

#[derive(bon::Builder)]
pub struct BuiltVisitor<T, S, L, G>
where
    S: FnMut(&mut T, FieldNumber, ScalarField),
    L: FnMut(&mut T, FieldNumber, Box<dyn DynLengthDelimited + '_>),
    G: FnMut(&mut T, FieldNumber, GroupOp),
{
    #[builder(start_fn)]
    state: T,
    on_scalar: S,
    on_length_delimited: L,
    on_group_op: G,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum GroupOp {
    Start,
    End,
}

pub trait DynLengthDelimited<'a> {
    fn into_bytes(self: Box<Self>) -> Result<Vec<u8>, DecodeError<Box<dyn std::error::Error>>>;

    fn into_packed_varints(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u64, DecodeError<Box<dyn std::error::Error>>>> + 'a>;

    fn into_packed_i32s(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u32, DecodeError<Box<dyn std::error::Error>>>> + 'a>;

    fn into_packed_i64s(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u64, DecodeError<Box<dyn std::error::Error>>>> + 'a>;

    fn into_packed(
        self: Box<Self>,
        wire_type: PackedWireType,
    ) -> Box<dyn Iterator<Item = Result<ScalarField, DecodeError<Box<dyn std::error::Error>>>> + 'a>;
}

pub enum PackedWireType {
    Varint,
    I32,
    I64,
}

struct DynLengthDelimitedImpl<L>(L);

impl<'a, L: VisitMessage + LengthDelimited + 'a> DynLengthDelimitedImpl<L> {
    fn into_packed_iter<W: ScalarWireType>(
        self: Box<Self>,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Box<dyn std::error::Error>>>> + 'a
    where
        L::PackedIter<W>: 'a,
    {
        self.0
            .into_packed::<W>()
            .map(move |r| r.map_err(|e| DecodeError::map_read(e, |e| Box::new(e) as Box<_>)))
    }
}

impl<'a, L: VisitMessage + LengthDelimited + 'a> DynLengthDelimited<'a>
    for DynLengthDelimitedImpl<L>
{
    fn into_bytes(self: Box<Self>) -> Result<Vec<u8>, DecodeError<Box<dyn std::error::Error>>> {
        let DynLengthDelimitedImpl(inner) = *self;

        let buffer = inner.into_bytes().map_err(|e| match e {
            DecodeError::Read(e) => DecodeError::Read(Box::new(e) as Box<dyn std::error::Error>),
            DecodeError::UnexpectedEnd => DecodeError::UnexpectedEnd,
            DecodeError::UnterminatedVarint => DecodeError::UnterminatedVarint,
            DecodeError::InvalidWireType(w) => DecodeError::InvalidWireType(w),
            DecodeError::TooLargeLengthDelimited(l) => DecodeError::TooLargeLengthDelimited(l),
        })?;
        Ok(buffer.as_ref().into())
    }

    fn into_packed_varints(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u64, DecodeError<Box<dyn std::error::Error>>>> + 'a> {
        Box::new(self.into_packed_iter::<Varint>())
    }

    fn into_packed_i32s(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u32, DecodeError<Box<dyn std::error::Error>>>> + 'a> {
        Box::new(self.into_packed_iter::<I32>())
    }

    fn into_packed_i64s(
        self: Box<Self>,
    ) -> Box<dyn Iterator<Item = Result<u64, DecodeError<Box<dyn std::error::Error>>>> + 'a> {
        Box::new(self.into_packed_iter::<I64>())
    }

    fn into_packed(
        self: Box<Self>,
        wire_type: PackedWireType,
    ) -> Box<dyn Iterator<Item = Result<ScalarField, DecodeError<Box<dyn std::error::Error>>>> + 'a>
    {
        Box::new(match wire_type {
            PackedWireType::Varint => Either::Left(
                self.into_packed_iter::<Varint>()
                    .map(move |r| r.map(ScalarField::Varint)),
            ),
            PackedWireType::I32 => Either::Right(Either::Left(
                self.into_packed_iter::<I32>()
                    .map(move |r| r.map(ScalarField::I32)),
            )),
            PackedWireType::I64 => Either::Right(Either::Right(
                self.into_packed_iter::<I64>()
                    .map(move |r| r.map(ScalarField::I64)),
            )),
        })
    }
}

impl<T, S, L, G> Visitor for BuiltVisitor<T, S, L, G>
where
    S: FnMut(&mut T, FieldNumber, ScalarField),
    L: FnMut(&mut T, FieldNumber, Box<dyn DynLengthDelimited + '_>),
    G: FnMut(&mut T, FieldNumber, GroupOp),
{
    fn on_scalar(&mut self, field_number: FieldNumber, field: ScalarField) {
        (self.on_scalar)(&mut self.state, field_number, field)
    }

    fn on_length_delimited<'s>(
        &'s mut self,
        field_number: FieldNumber,
        handler: impl VisitMessage + LengthDelimited + 's,
    ) {
        (self.on_length_delimited)(
            &mut self.state,
            field_number,
            Box::new(DynLengthDelimitedImpl(handler)),
        )
    }

    fn on_group_begin(&mut self, field_number: FieldNumber) {
        (self.on_group_op)(&mut self.state, field_number, GroupOp::Start)
    }

    fn on_group_end(&mut self, field_number: FieldNumber) {
        (self.on_group_op)(&mut self.state, field_number, GroupOp::End)
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn extract_single_string() {
        let input = [0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67];

        let mut extracted = None;

        let visitor = BuiltVisitor::builder(&mut extracted)
            .on_length_delimited(|extracted, _field_number, delimited| {
                assert_matches!(
                    extracted.replace(delimited.into_bytes().expect("can read")),
                    None
                );
            })
            .on_scalar(|_, _, _| ())
            .on_group_op(|_, _, _| ())
            .build();

        let result =
            crate::visitor::visit_message(crate::wire::parse(&mut input.as_slice()), visitor);
        assert_matches!(result, Ok(()));
        assert_eq!(extracted, Some("testing".to_string().into()))
    }
}
