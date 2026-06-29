#![doc(hidden)]

#[cfg(feature = "std")]
use core::hash::Hash;
#[cfg(any(feature = "std", doc))]
use std::collections::HashMap;

use crate::read::ReadTypes;
use crate::scan::encoding::Encoding;
use crate::scan::field::map::{MapEntry, MapEntryScanner};
use crate::scan::field::save::bytes::SaveBytesScanner;
use crate::scan::field::{Map, MapKey, OnScanField, Save};
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, ScanError, ScanLengthDelimited, ScanMessage,
};
#[cfg(feature = "std")]
use crate::scan::{IntoResettableScanner, ResettableScanner};
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
    /// Returns an [`IntoScanner`] impl that saves only the scanned value for
    /// the given key from the map.
    pub fn map_value<K, SV>(key: K, value_scanner: SV) -> SaveMapValue<K, SV> {
        SaveMapValue(key, value_scanner)
    }
}

impl<K: AsRef<str>, SV> SaveMapValue<K, SV> {
    #[inline]
    pub fn skip_keys_utf8_validation(self) -> SaveMapValue<SkipUtf8Validation<K>, SV> {
        SaveMapValue(SkipUtf8Validation(self.0), self.1)
    }
}

pub struct SkipUtf8Validation<K>(K);

impl<K: AsRef<str>, B: AsRef<[u8]>> PartialEq<B> for SkipUtf8Validation<K> {
    #[inline]
    fn eq(&self, other: &B) -> bool {
        self.0.as_ref().as_bytes() == other.as_ref()
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>> IntoScanner<Map<K, V>> for SaveMap<SV>
where
    Save: IntoScanner<K>,
{
    type Scanner<R: ReadTypes> = SaveMapScanner<K, V, R, SV>;

    #[inline]
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapScanner {
            output: HashMap::new(),
            value_scanner: self.0,
        }
    }
}

#[allow(type_alias_bounds)]
#[cfg(feature = "std")]
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

    #[inline]
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
    #[inline]
    fn on_numeric(&mut self, _value: NumericField) -> Result<(), ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    #[inline]
    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    #[inline]
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
    SK: IntoScanner<K>,
    SV: IntoScanner<V, Scanner<R>: IntoScanOutput>,
> {
    needle: Q,
    scanner: MapEntryScanner<K, V, SK, SV>,
    found: Option<<SV::Scanner<R> as IntoScanOutput>::ScanOutput>,
}

pub trait SaveMapKey<K: ?Sized> {
    type SaveScanner: IntoScanner<K>;
    fn save_scanner() -> Self::SaveScanner;
}

impl<T: MapKey + Encoding> SaveMapKey<T> for T::Repr
where
    Save: IntoScanner<T>,
{
    type SaveScanner = <T as MapKey>::SaveScanner;
    fn save_scanner() -> Self::SaveScanner {
        <T as MapKey>::save_scanner()
    }
}

impl<S: AsRef<str>> SaveMapKey<str> for S {
    type SaveScanner = Save;
    fn save_scanner() -> Self::SaveScanner {
        Save
    }
}

impl<S: AsRef<str>> SaveMapKey<str> for SkipUtf8Validation<S> {
    type SaveScanner = SaveBytes;
    fn save_scanner() -> Self::SaveScanner {
        SaveBytes
    }
}

#[derive(Clone)]
pub struct SaveBytes;

impl IntoScanner<str> for SaveBytes {
    type Scanner<R: ReadTypes> = SaveBytesScanner<[u8], R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner::new()
    }
}

impl<K: MapKey + ?Sized, V: ?Sized, Q: SaveMapKey<K>, SV: IntoScanner<V>> IntoScanner<Map<K, V>>
    for SaveMapValue<Q, SV>
{
    type Scanner<R: ReadTypes> = SaveMapValueScanner<K, V, R, Q, Q::SaveScanner, SV>;

    #[inline]
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapValueScanner {
            needle: self.0,
            scanner: MapEntry::scanner().key(Q::save_scanner()).value(self.1),
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
    SK: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput> + Clone,
    SV: IntoScanner<V, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput = O>> + Clone,
> OnScanField<R> for SaveMapValueScanner<K, V, R, Q, SK, SV>
where
    Q: PartialEq<<SK::Scanner<R> as IntoScanOutput>::ScanOutput>,
{
    #[inline]
    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        Err(ScanError::WrongWireType)
    }

    #[inline]
    fn on_group(
        &mut self,
        _group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        Err(ScanError::WrongWireType)
    }

    #[inline]
    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R as ReadTypes>::Error>> {
        let scanner = self.scanner.clone();
        let (key, value) =
            delimited.scan_with(IntoScanner::<MapEntry<K, V>>::into_scanner(scanner))?;

        if self.needle == key {
            self.found = Some(value);
        }
        Ok(())
    }
}

impl<K: ?Sized, V: ?Sized, R: ReadTypes, Q, SK: IntoScanner<K>, SV: IntoScanner<V>> IntoScanOutput
    for SaveMapValueScanner<K, V, R, Q, SK, SV>
{
    type ScanOutput = Option<<SV::Scanner<R> as IntoScanOutput>::ScanOutput>;
    #[inline]
    fn into_scan_output(self) -> Self::ScanOutput {
        self.found
    }
}
