use std::convert::Infallible;

use crate::read::ReadBuffer as _;
use crate::scan::encoding::Encoding;
use crate::scan::field::OnScanField;
use crate::scan::{IntoResettable, IntoScanOutput, IntoScanner, Resettable, StopScan};
use crate::wire::{GroupOp, LengthDelimited, NumericField, NumericWireType};

/// [`OnScanField`] implementation that produces the read value as the event output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct SaveNumeric<E: Encoding>(Option<E::Repr>);

impl<T: Encoding> SaveNumeric<T> {
    pub fn new() -> Self {
        Self(None)
    }
}

impl<E: Encoding> OnScanField for SaveNumeric<E> {
    type ScanEvent = E::Repr;

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
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

impl<E: Encoding> Resettable for SaveNumeric<E> {
    fn reset(&mut self) {
        self.0 = None;
    }
}

impl<E: Encoding> IntoResettable for SaveNumeric<E> {
    type Resettable = Self;
    fn into_resettable(self) -> Self::Resettable {
        self
    }
}

impl<E: Encoding> IntoScanner for SaveNumeric<E> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}

impl<E: Encoding> IntoScanOutput for SaveNumeric<E> {
    type ScanOutput = Option<E::Repr>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

/// [`OnScanField`] implementation that produces the read values as the scan output.
///
/// Deserializes according to the [`Encoding`] type parameter.
pub struct SaveRepeated<E: Encoding>(Vec<E::Repr>);

impl<T: Encoding> SaveRepeated<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<E: Encoding> OnScanField for SaveRepeated<E> {
    type ScanEvent = Infallible;

    fn on_numeric(&mut self, value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        let value = <E::Wire as NumericWireType>::from_value(value).ok_or(StopScan)?;
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

impl<E: Encoding> IntoScanOutput for SaveRepeated<E> {
    type ScanOutput = Vec<E::Repr>;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: Encoding> Resettable for SaveRepeated<E> {
    fn reset(&mut self) {
        self.0.clear()
    }
}

impl<E: Encoding> IntoScanner for SaveRepeated<E> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}

/// [`OnScanField`] implementation that produces the read value as the event output.
pub struct SaveBytes<E: ToOwnedBytes + ?Sized>(Option<E::Owned>);

pub trait ToOwnedBytes {
    type Owned: for<'a> From<&'a Self>;
}

impl ToOwnedBytes for str {
    type Owned = String;
}

impl ToOwnedBytes for [u8] {
    type Owned = Box<[u8]>;
}

impl<T: ToOwnedBytes + ?Sized> SaveBytes<T> {
    pub fn new() -> Self {
        Self(None)
    }
}

impl OnScanField for SaveBytes<str> {
    type ScanEvent = Infallible;
    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        let bytes = delimited.into_bytes().map_err(|_| StopScan)?.into_bytes();
        let string = String::from_utf8(bytes.into()).map_err(|_| StopScan)?;
        self.0 = Some(string);
        Ok(None)
    }
}

impl IntoScanOutput for SaveBytes<str> {
    type ScanOutput = Option<String>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl OnScanField for SaveBytes<[u8]> {
    type ScanEvent = Infallible;

    fn on_numeric(&mut self, _value: NumericField) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        let bytes = delimited.into_bytes().map_err(|_| StopScan)?.into_bytes();
        self.0 = Some(bytes);
        Ok(None)
    }
}

impl IntoScanOutput for SaveBytes<[u8]> {
    type ScanOutput = Option<Box<[u8]>>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

impl<E: ToOwnedBytes + ?Sized> Resettable for SaveBytes<E> {
    fn reset(&mut self) {
        self.0 = None;
    }
}

impl<E: ToOwnedBytes + ?Sized> IntoScanner for SaveBytes<E> {
    type Scanner = Self;
    fn into_scanner(self) -> Self::Scanner {
        self
    }
}
