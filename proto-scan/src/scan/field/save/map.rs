#![doc(hidden)]

#[cfg(any(feature = "std", doc))]
use std::collections::HashMap;
#[cfg(feature = "std")]
use {core::convert::Infallible, core::hash::Hash, core::marker::PhantomData};

use either::Either;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
#[cfg(feature = "std")]
use crate::scan::field::{Map, MapKey, Save};
use crate::scan::{GroupDelimited, IntoScanOutput, ScanCallbacks, ScanError, ScanLengthDelimited};
#[cfg(feature = "std")]
use crate::scan::{IntoResettableScanner, IntoScanner, ResettableScanner};
use crate::wire::NumericField;

/// Saves map keys and the output of a provided value scanner.
///
/// Implements [`IntoScanner`]; the provided scanner produces as output a
/// [`HashMap`] of keys from the map to the values.
pub struct SaveMap<V>(V);

impl<V> SaveMap<V> {
    pub fn with_value(value_scanner: V) -> Self {
        Self(value_scanner)
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>> IntoScanner<Map<K, V>> for SaveMap<SV>
where
    Save: IntoScanner<K>,
{
    type Scanner<R: ReadTypes> = SaveMapScanner<K, SV::Scanner<R>, R>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapScanner(HashMap::new(), self.0.into_scanner(), PhantomData)
    }
}

#[cfg(feature = "std")]
pub struct SaveMapScanner<K: MapKey + ?Sized, SV: IntoScanOutput, R: ReadTypes>(
    HashMap<<<Save as IntoScanner<K>>::Scanner<R> as IntoScanOutput>::ScanOutput, SV::ScanOutput>,
    SV,
    PhantomData<R>,
)
where
    Save: IntoScanner<K>;

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, SV: IntoScanOutput, R: ReadTypes> IntoScanOutput
    for SaveMapScanner<K, SV, R>
where
    Save: IntoScanner<K>,
{
    type ScanOutput = HashMap<
        <<Save as IntoScanner<K>>::Scanner<R> as IntoScanOutput>::ScanOutput,
        SV::ScanOutput,
    >;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, SV: OnScanField<R> + Clone, R: ReadTypes> OnScanField<R>
    for SaveMapScanner<K, SV, R>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: Hash + Eq>>,
{
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        let Self(map, value_scanner, PhantomData) = self;
        let mut scanner = MapEntry(IntoScanner::<K>::into_scanner(Save), value_scanner.clone());
        delimited.scan_with(&mut scanner)?;
        let MapEntry(key, value) = scanner;
        map.insert(key.into_scan_output(), value.into_scan_output());
        Ok(None)
    }
}

#[cfg(feature = "std")]
impl<
    K: MapKey + ?Sized,
    SV: OnScanField<R> + IntoResettableScanner<Resettable: IntoScanOutput>,
    R: ReadTypes,
> IntoResettableScanner for SaveMapScanner<K, SV, R>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: Hash + Eq>>,
{
    type Resettable = SaveMapScanner<K, SV::Resettable, R>;
    fn into_resettable(self) -> Self::Resettable {
        let Self(_map, sv, PhantomData) = self;
        SaveMapScanner(HashMap::new(), sv.into_resettable(), PhantomData)
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, SV: IntoScanOutput, R: ReadTypes> ResettableScanner
    for SaveMapScanner<K, SV, R>
where
    Save: IntoScanner<K>,
{
    fn reset(&mut self) {
        self.0.clear();
    }
}

/// Synthetic scanner for the wire representation of a protobuf map entry.
///
/// Field 1 is the key, field 2 is the value. This exists to be used with the
/// existing scan machinery, but could otherwise be inlined above.
struct MapEntry<SK, SV>(SK, SV);

impl<SK: IntoScanOutput, SV: IntoScanOutput> IntoScanOutput for MapEntry<SK, SV> {
    type ScanOutput = (SK::ScanOutput, SV::ScanOutput);
    fn into_scan_output(self) -> Self::ScanOutput {
        (self.0.into_scan_output(), self.1.into_scan_output())
    }
}

impl<SK: OnScanField<R>, SV: OnScanField<R>, R: ReadTypes> ScanCallbacks<R> for MapEntry<SK, SV> {
    type ScanEvent = Option<Either<SK::ScanEvent, SV::ScanEvent>>;

    fn on_numeric(
        &mut self,
        field: crate::wire::FieldNumber,
        value: NumericField,
    ) -> Result<Self::ScanEvent, ScanError<<R>::Error>> {
        Ok(match u32::from(field) {
            1 => self.0.on_numeric(value)?.map(Either::Left),
            2 => self.1.on_numeric(value)?.map(Either::Right),
            _ => None,
        })
    }

    fn on_group(
        &mut self,
        field: crate::wire::FieldNumber,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, ScanError<<R>::Error>> {
        Ok(match u32::from(field) {
            1 => self.0.on_group(group)?.map(Either::Left),
            2 => self.1.on_group(group)?.map(Either::Right),
            _ => None,
        })
    }

    fn on_length_delimited(
        &mut self,
        field: crate::wire::FieldNumber,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, ScanError<<R>::Error>> {
        Ok(match u32::from(field) {
            1 => self.0.on_length_delimited(delimited)?.map(Either::Left),
            2 => self.1.on_length_delimited(delimited)?.map(Either::Right),
            _ => None,
        })
    }
}
