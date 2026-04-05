use crate::DecodeError;
use crate::read::ReadTypes;
use crate::wire::{FieldNumber, LengthDelimited, NumericField, ParseEventReader};

/// Callback interface for handling a stream of parse events.
/// 
/// This provides a push-based interface for handling parse events. That's in
/// contrast to [`ParseEventReader::next`], which is a pull-based mechanism.
/// 
/// This can be used with [`ParseEventReader::read_all`] to receive callbacks
/// for each parsed tag.
pub trait ParseCallbacks<R: ReadTypes> {
    type ParseError: From<DecodeError<R::Error>>;

    /// Called when a numeric field is parsed.
    fn on_numeric(
        &mut self,
        field: FieldNumber,
        value: NumericField,
    ) -> Result<(), Self::ParseError>;

    /// Called when a SGROUP tag is read.
    fn on_group_start(
        &mut self,
        field: FieldNumber,
        parse: &mut impl ParseEventReader<ReadTypes = R>,
    ) -> Result<(), Self::ParseError>;

    /// Called when a EGROUP tag is read.
    fn on_group_end(&mut self, field: FieldNumber) -> Result<(), Self::ParseError>;

    /// Called when a length-delimited field tag is encountered.
    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<(), Self::ParseError>;
}
