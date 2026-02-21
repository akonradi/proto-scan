use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{ScanTypes, StopScan};
use crate::wire::{GroupOp, LengthDelimited, ScalarField, ScalarWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct EmitScalar<E: Encoding>(Option<E::Repr>);

impl<E: Encoding> ScanTypes for EmitScalar<E> {
    type ScanEvent = E::Repr;
    type ScanOutput = Option<E::Repr>;
}

impl<E: Encoding> OnScanField for EmitScalar<E> {
    fn into_output(self) -> Self::ScanOutput {
        self.0
    }
    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
        let value = E::decode(value).map_err(Into::into)?;
        self.0 = Some(value);
        Ok(Some(value))
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

impl<T: Encoding> EmitScalar<T> {
    pub fn new() -> Self {
        Self(None)
    }
}
