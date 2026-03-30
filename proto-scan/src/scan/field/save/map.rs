#![doc(hidden)]

#[cfg(feature = "std")]
use core::hash::Hash;
#[cfg(any(feature = "std", doc))]
use std::collections::HashMap;

#[cfg(feature = "std")]
use crate::read::ReadTypes;
use crate::scan::ScanMessage;
use crate::scan::field::map::MapEntry;
#[cfg(feature = "std")]
use crate::scan::field::map::MapEntryScanner;
#[cfg(feature = "std")]
use crate::scan::field::{Map, MapKey, OnScanField, Save};
#[cfg(feature = "std")]
use crate::scan::{
    GroupDelimited, IntoResettableScanner, IntoScanOutput, IntoScanner, ResettableScanner,
    ScanError, ScanLengthDelimited,
};
#[cfg(feature = "std")]
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

/// Saves a single value from a map.
///
/// Implements [`IntoScanner`]; the provided scanner saves only the scanned
/// value for the provided key from the map (if present).
pub struct SaveMapValue<K, SV>(K, SV);

impl super::Save {
    /// Saves a single value from a map.
    ///
    /// Returns an [`IntoScanner`] impl thatl saves only the scanned value for
    /// the given key from the map.
    pub fn map_value<K, SV>(key: K, value_scanner: SV) -> SaveMapValue<K, SV> {
        SaveMapValue(key, value_scanner)
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>> IntoScanner<Map<K, V>> for SaveMap<SV>
where
    Save: IntoScanner<K>,
{
    type Scanner<R: ReadTypes> = SaveMapScanner<K, V, R, SV>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapScanner {
            output: HashMap::new(),
            value_scanner: self.0,
        }
    }
}

#[allow(type_alias_bounds)]
type ScannerOutput<S: IntoScanner<M, Scanner<R>: IntoScanOutput>, M, R> =
    <<S as IntoScanner<M>>::Scanner<R> as IntoScanOutput>::ScanOutput;

#[cfg(feature = "std")]
pub struct SaveMapScanner<
    K: MapKey + ?Sized,
    V: ?Sized,
    R: ReadTypes,
    SV: IntoScanner<V, Scanner<R>: IntoScanOutput>,
> where
    Save: IntoScanner<K>,
{
    output: HashMap<ScannerOutput<Save, K, R>, ScannerOutput<SV, V, R>>,
    value_scanner: SV,
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>, R: ReadTypes> IntoScanOutput
    for SaveMapScanner<K, V, R, SV>
where
    Save: IntoScanner<K>,
{
    type ScanOutput = HashMap<
        <<Save as IntoScanner<K>>::Scanner<R> as IntoScanOutput>::ScanOutput,
        <<SV as IntoScanner<V>>::Scanner<R> as IntoScanOutput>::ScanOutput,
    >;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.output
    }
}

#[cfg(feature = "std")]
impl<
    K: MapKey + ?Sized,
    V: ?Sized,
    SV: Clone + IntoScanner<V, Scanner<R>: OnScanField<R>>,
    R: ReadTypes,
> OnScanField<R> for SaveMapScanner<K, V, R, SV>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: Hash + Eq>>,
{
    fn on_numeric(&mut self, _value: NumericField) -> Result<(), ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        let Self {
            output: map,
            value_scanner,
        } = self;
        let scanner = IntoScanner::<MapEntry<K, V>>::into_scanner(
            MapEntry::scanner().key(Save).value(value_scanner.clone()),
        );
        let (key, value) = delimited.scan_with(scanner)?;
        map.insert(key, value);
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<
    K: MapKey + ?Sized,
    V: ?Sized,
    SV: IntoScanner<V> + IntoResettableScanner<Resettable: IntoScanner<V>>,
    R: ReadTypes,
> IntoResettableScanner for SaveMapScanner<K, V, R, SV>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: Hash + Eq>>,
{
    type Resettable = SaveMapScanner<K, V, R, SV::Resettable>;
    fn into_resettable(self) -> Self::Resettable {
        let Self {
            output: _map,
            value_scanner: sv,
        } = self;
        SaveMapScanner {
            output: HashMap::new(),
            value_scanner: sv.into_resettable(),
        }
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>, R: ReadTypes> ResettableScanner
    for SaveMapScanner<K, V, R, SV>
where
    Save: IntoScanner<K>,
{
    fn reset(&mut self) {
        self.output.clear();
    }
}

pub struct SaveMapValueScanner<
    K: ?Sized,
    V: ?Sized,
    R: ReadTypes,
    Q,
    SV: IntoScanner<V, Scanner<R>: IntoScanOutput>,
> {
    needle: Q,
    scanner: MapEntryScanner<K, V, Save, SV>,
    found: Option<<SV::Scanner<R> as IntoScanOutput>::ScanOutput>,
}

impl<K: MapKey + ?Sized, V: ?Sized, Q, SV: IntoScanner<V>> IntoScanner<Map<K, V>>
    for SaveMapValue<Q, SV>
where
    Save: IntoScanner<K>,
{
    type Scanner<R: ReadTypes> = SaveMapValueScanner<K, V, R, Q, SV>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapValueScanner {
            needle: self.0,
            scanner: MapEntry::scanner().key(Save).value(self.1),
            found: None,
        }
    }
}

impl<
    R: ReadTypes,
    K: MapKey + ?Sized,
    V: ?Sized,
    Q,
    O,
    SV: IntoScanner<V, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput = O>> + Clone,
> OnScanField<R> for SaveMapValueScanner<K, V, R, Q, SV>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: PartialEq<Q>>>,
{
    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        let scanner = self.scanner.clone();
        let (key, value) =
            delimited.scan_with(IntoScanner::<MapEntry<K, V>>::into_scanner(scanner))?;

        if key == self.needle {
            self.found = Some(value);
        }
        Ok(())
    }
}

impl<K: ?Sized, V: ?Sized, R: ReadTypes, Q, SV: IntoScanner<V>> IntoScanOutput
    for SaveMapValueScanner<K, V, R, Q, SV>
{
    type ScanOutput = Option<<SV::Scanner<R> as IntoScanOutput>::ScanOutput>;
    fn into_scan_output(self) -> Self::ScanOutput {
        self.found
    }
}
