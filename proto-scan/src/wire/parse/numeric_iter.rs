use std::marker::PhantomData;

use crate::decode_error::DecodeError;
use crate::read::Read;
use crate::wire::NumericWireType;
use crate::wire::parse::{DoBeforeNext, LengthDelimitedImpl};

pub(super) struct NumericIter<'a, R, W> {
    inner: LengthDelimitedImpl<'a, R>,
    _wire_type: PhantomData<W>,
}

impl<'a, R, W> NumericIter<'a, R, W> {
    pub(crate) fn new(inner: LengthDelimitedImpl<'a, R>) -> Self {
        Self {
            inner,
            _wire_type: PhantomData::<W>,
        }
    }
}

impl<'a, R: Read, W: NumericWireType> Iterator for NumericIter<'a, R, W> {
    type Item = Result<W::Repr, DecodeError<R::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let Self { inner, _wire_type } = self;
        if inner.reader.remaining() == 0 {
            return None;
        }

        Some(match W::read_from(inner) {
            Ok(r) => Ok(r),
            Err(e) => {
                *self.inner.write_back_to = DoBeforeNext::Error;
                Err(e)
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.inner.reader.remaining();
        let (min, max) = W::BYTE_LEN.into_inner();

        (
            (remaining / u32::from(max)).try_into().unwrap_or(0),
            (remaining.div_ceil(min.into())).try_into().ok(),
        )
    }
}
