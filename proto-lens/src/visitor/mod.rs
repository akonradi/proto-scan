use crate::DecodeError;
use crate::wire::{FieldNumber, LengthDelimited, ParseEvent, ParseEventReader, ScalarField};

mod build;
pub use build::Builder;

mod visit_message;
use visit_message::VisitMessageImpl;

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
) -> Result<(), DecodeError<P::Error>> {
    while let Some(event) = reader.next() {
        let (field_number, event) = event?;
        match event {
            ParseEvent::Scalar(value) => visitor.on_scalar(field_number, value),
            ParseEvent::StartGroup => visitor.on_group_begin(field_number),
            ParseEvent::EndGroup => visitor.on_group_end(field_number),
            ParseEvent::LengthDelimited(length_delimited) => {
                let mut result = Ok(());
                visitor.on_length_delimited(
                    field_number,
                    VisitMessageImpl {
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
                    extracted.replace(delimited.into_bytes().expect("can read")),
                    None
                );
            })
            .build();

        let result = visit_message(crate::wire::parse(&mut input.as_slice()), visitor);
        assert_matches!(result, Ok(()));
        assert_eq!(extracted, Some("testing".to_string().into()))
    }
}
