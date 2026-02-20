use std::marker::PhantomData;

use crate::scan::StopScan;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::wire::{GroupOp, LengthDelimited, ScalarField, ScalarWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct EmitScalar<E>(PhantomData<E>);

impl<E: Encoding> OnScanField for EmitScalar<E> {
    type ScanEvent = E::Repr;

    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
        E::decode(value).map(Some).map_err(Into::into)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        _delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }
}

impl<T> EmitScalar<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
