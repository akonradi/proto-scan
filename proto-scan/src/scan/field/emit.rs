use std::convert::Infallible;

use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{Resettable, ScanTypes, StopScan};
use crate::wire::{GroupOp, LengthDelimited, ScalarField, ScalarWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct EmitScalar<E: Encoding>(Option<E::Repr>);

impl<E: Encoding> ScanTypes for EmitScalar<E> {
    type ScanEvent = E::Repr;
    type ScanOutput = Option<E::Repr>;
}

impl<T: Encoding> EmitScalar<T> {
    pub fn new() -> Self {
        Self(None)
    }
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

impl<E: Encoding> Resettable for EmitScalar<E> {
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {}
    fn reset(&mut self, to: ()) {
        self.0 = None;
    }
}

/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct EmitRepeated<E: Encoding>(Vec<E::Repr>);

impl<E: Encoding> ScanTypes for EmitRepeated<E> {
    type ScanEvent = Infallible;
    type ScanOutput = Vec<E::Repr>;
}

impl<T: Encoding> EmitRepeated<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<E: Encoding> OnScanField for EmitRepeated<E> {
    fn into_output(self) -> Self::ScanOutput {
        self.0
    }
    fn on_scalar(&mut self, value: ScalarField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as ScalarWireType>::from_value(value).ok_or(StopScan)?;
        self.0.extend([E::decode(value).map_err(Into::into)?]);
        Ok(None)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        let mut packed = delimited.into_packed::<E::Wire>();
        let mut result = Ok(None);
        self.0.extend(core::iter::from_fn(|| {
            let value = packed.next()?.ok().and_then(|w| E::decode(w).ok());
            if value.is_none() {
                result = Err(StopScan);
            }
            value
        }));
        result
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }
}

impl<E: Encoding> Resettable for EmitRepeated<E> {
    type Mark = ();
    fn mark(&mut self) -> Self::Mark {}
    fn reset(&mut self, (): Self::Mark) {
        self.0.clear()
    }
}
