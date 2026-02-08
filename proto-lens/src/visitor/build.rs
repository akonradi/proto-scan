use crate::DecodeError;
use crate::visitor::{VisitMessage, ScalarField, Visitor};
use crate::wire::FieldNumber;

pub struct Builder<T> {
    state: T,
    on_scalar: Box<dyn FnMut(&mut T, FieldNumber, ScalarField)>,
    on_length_delimited:
        Box<dyn for<'s> FnMut(&'s mut T, FieldNumber, Box<dyn DynLengthDelimited + 's>)>,
    on_group_begin: Box<dyn FnMut(&mut T, FieldNumber)>,
    on_group_end: Box<dyn FnMut(&mut T, FieldNumber)>,
}

pub trait DynLengthDelimited {
    fn as_bytes(self: Box<Self>) -> Result<Vec<u8>, DecodeError<Box<dyn std::error::Error>>>;
}

struct DynLengthDelimitedImpl<L>(L);

impl<L: VisitMessage + crate::wire::LengthDelimited> DynLengthDelimited
    for DynLengthDelimitedImpl<L>
{
    fn as_bytes(self: Box<Self>) -> Result<Vec<u8>, DecodeError<Box<dyn std::error::Error>>> {
        let DynLengthDelimitedImpl(inner) = *self;

        let buffer = inner.as_bytes().map_err(|e| match e {
            DecodeError::Read(e) => DecodeError::Read(Box::new(e) as Box<dyn std::error::Error>),
            DecodeError::UnexpectedEnd => DecodeError::UnexpectedEnd,
            DecodeError::UnterminatedVarint => DecodeError::UnterminatedVarint,
            DecodeError::InvalidWireType(w) => DecodeError::InvalidWireType(w),
            DecodeError::TooLargeLengthDelimited(l) => DecodeError::TooLargeLengthDelimited(l),
        })?;
        Ok(buffer.as_ref().into())
    }
}

impl<T> Builder<T> {
    pub fn new(state: T) -> Self {
        Self {
            state,
            on_scalar: Box::new(|_, _, _| {}),
            on_length_delimited: Box::new(|_, _, _| {}),
            on_group_begin: Box::new(|_, _| {}),
            on_group_end: Box::new(|_, _| {}),
        }
    }

    pub fn set_on_scalar(
        mut self,
        f: impl FnMut(&mut T, FieldNumber, ScalarField) + 'static,
    ) -> Self {
        self.on_scalar = Box::new(f);
        self
    }

    pub fn set_on_length_delimited(
        mut self,
        f: impl for<'s> FnMut(&'s mut T, FieldNumber, Box<dyn DynLengthDelimited + 's>) + 'static,
    ) -> Self {
        self.on_length_delimited = Box::new(f);
        self
    }

    pub fn set_on_group_begin(mut self, f: impl FnMut(&mut T, FieldNumber) + 'static) -> Self {
        self.on_group_begin = Box::new(f);
        self
    }
    pub fn set_on_group_end(mut self, f: impl FnMut(&mut T, FieldNumber) + 'static) -> Self {
        self.on_group_end = Box::new(f);
        self
    }

    pub fn build(self) -> impl Visitor {
        self
    }
}

impl<T> Visitor for Builder<T> {
    fn on_scalar(&mut self, field_number: FieldNumber, field: ScalarField) {
        (self.on_scalar)(&mut self.state, field_number, field)
    }

    fn on_length_delimited<'s>(
        &'s mut self,
        field_number: FieldNumber,
        handler: impl VisitMessage + crate::wire::LengthDelimited + 's,
    ) {
        (self.on_length_delimited)(
            &mut self.state,
            field_number,
            Box::new(DynLengthDelimitedImpl(handler)),
        )
    }

    fn on_group_begin(&mut self, field_number: FieldNumber) {
        (self.on_group_begin)(&mut self.state, field_number)
    }

    fn on_group_end(&mut self, field_number: FieldNumber) {
        (self.on_group_end)(&mut self.state, field_number)
    }
}
