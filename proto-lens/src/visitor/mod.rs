use crate::DecodeError;
use crate::wire::{
    FieldNumber, LengthDelimited, ParseEvent, ParseEventReader, ScalarField, ScalarWireType,
};

mod build;
pub use build::Builder;

/// Visitor for a serialized protobuf message.
/// 
/// Implementers can be passed as an argument to [`visit_message`] to receive
/// callbacks for each field in the message.
pub trait Visitor {
    /// Called when a scalar field is parsed.
    fn on_scalar(&mut self, field_number: FieldNumber, field: ScalarField);

    /// Called when a length-delimited field is parsed.
    /// 
    /// Implementations can use the provided handler argument to access the
    /// contents of the field.
    fn on_length_delimited<'s>(
        &'s mut self,
        field_number: FieldNumber,
        handler: impl VisitMessage + LengthDelimited + 's,
    );

    /// Called when a SGROUP tag is found.
    fn on_group_begin(&mut self, field_number: FieldNumber);

    /// Called when a EGROUP tag is found.
    fn on_group_end(&mut self, field_number: FieldNumber);
}

/// Allows visiting the contents of a length-delimited message with a
/// [`Visitor`].
pub trait VisitMessage {
    fn visit_message(self, visitor: impl Visitor);
}

pub fn visit_message<P: ParseEventReader>(
    mut reader: P,
    mut visitor: impl Visitor,
) -> Result<(), DecodeError<P::ReadError>> {
    while let Some(event) = reader.next() {
        match event? {
            ParseEvent::Scalar(field_number, value) => visitor.on_scalar(field_number, value),
            ParseEvent::StartGroup(field_number) => visitor.on_group_begin(field_number),
            ParseEvent::EndGroup(field_number) => visitor.on_group_end(field_number),
            ParseEvent::LengthDelimited(field_number, length_delimited) => {
                let mut result = Ok(());
                visitor.on_length_delimited(
                    field_number,
                    LengthDelimitedImpl {
                        inner: length_delimited,
                        result: &mut result,
                    },
                );
                result?;
            }
        }
    }
    Ok(())
}

struct LengthDelimitedImpl<'a, L: LengthDelimited> {
    inner: L,
    result: &'a mut Result<(), DecodeError<L::ReadError>>,
}

impl<L: LengthDelimited> LengthDelimited for LengthDelimitedImpl<'_, L> {
    type ReadBuffer = L::ReadBuffer;
    type ReadError = L::ReadError;

    fn len(&self) -> u32 {
        self.inner.len()
    }

    fn as_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Self::ReadError>>> {
        self.inner.as_packed::<W>()
    }

    fn as_bytes(self) -> Result<Self::ReadBuffer, DecodeError<Self::ReadError>> {
        self.inner.as_bytes()
    }

    fn as_events(self) -> impl ParseEventReader<ReadError = Self::ReadError> {
        self.inner.as_events()
    }
}

impl<L: LengthDelimited> VisitMessage for LengthDelimitedImpl<'_, L> {
    fn visit_message(self, visitor: impl Visitor) {
        let reader = self.inner.as_events();
        *self.result = visit_message(reader, visitor);
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

        let visitor = Builder::new(&mut extracted)
            .set_on_length_delimited(|extracted, _field_number, delimited| {
                assert_matches!(
                    extracted.replace(delimited.as_bytes().expect("can read")),
                    None
                );
            })
            .build();

        let result = visit_message(crate::wire::parse(&mut input.as_slice()), visitor);
        assert_matches!(result, Ok(()));
        assert_eq!(extracted, Some("testing".to_string().into()))
    }
}
