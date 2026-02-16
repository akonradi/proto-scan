use crate::DecodeError;
use crate::read::{ReadError, ReadTypes};
use crate::visitor::{VisitMessage, Visitor};
use crate::wire::{LengthDelimited, ParseEventReader, ScalarWireType};

pub(super) struct VisitMessageImpl<'a, L: ReadError> {
    pub(super) inner: L,
    pub(super) result: &'a mut Result<(), DecodeError<L::Error>>,
}

impl<L: ReadError> ReadError for VisitMessageImpl<'_, L> {
    type Error = L::Error;
}

impl<L: ReadTypes> ReadTypes for VisitMessageImpl<'_, L> {
    type Buffer = L::Buffer;
}

impl<L: LengthDelimited> LengthDelimited for VisitMessageImpl<'_, L> {
    fn len(&self) -> u32 {
        self.inner.len()
    }

    fn into_packed<W: ScalarWireType>(
        self,
    ) -> impl Iterator<Item = Result<W::Repr, DecodeError<Self::Error>>> {
        self.inner.into_packed::<W>()
    }

    fn into_bytes(self) -> Result<Self::Buffer, DecodeError<Self::Error>> {
        self.inner.into_bytes()
    }

    fn into_events(self) -> impl ParseEventReader<Error = Self::Error> {
        self.inner.into_events()
    }
}

impl<L: LengthDelimited> VisitMessage for VisitMessageImpl<'_, L> {
    fn visit_message(self, visitor: impl Visitor) {
        let reader = self.inner.into_events();
        *self.result = super::visit_message(reader, visitor);
    }
}
