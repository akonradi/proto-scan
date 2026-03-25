#![doc(hidden)]

#[cfg(any(feature = "std", doc))]
use std::collections::HashMap;
#[cfg(feature = "std")]
use {core::hash::Hash, core::marker::PhantomData};

#[cfg(feature = "std")]
use crate::read::ReadTypes;
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

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>> IntoScanner<Map<K, V>> for SaveMap<SV>
where
    Save: IntoScanner<K>,
{
    type Scanner<R: ReadTypes> = SaveMapScanner<K, V, SV::Scanner<R>, R>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapScanner(
            HashMap::new(),
            self.0.into_scanner(),
            PhantomData,
            PhantomData,
        )
    }
}

#[cfg(feature = "std")]
pub struct SaveMapScanner<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanOutput, R: ReadTypes>(
    HashMap<<<Save as IntoScanner<K>>::Scanner<R> as IntoScanOutput>::ScanOutput, SV::ScanOutput>,
    SV,
    PhantomData<R>,
    PhantomData<V>,
)
where
    Save: IntoScanner<K>;

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanOutput, R: ReadTypes> IntoScanOutput
    for SaveMapScanner<K, V, SV, R>
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
impl<K: MapKey + ?Sized, V: ?Sized, SV: OnScanField<R> + Clone, R: ReadTypes> OnScanField<R>
    for SaveMapScanner<K, V, SV, R>
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
        let Self(map, value_scanner, PhantomData, PhantomData) = self;
        let mut scanner = MapEntryScanner::<K, V, _, _>::new(
            IntoScanner::<K>::into_scanner(Save),
            value_scanner.clone(),
        );
        delimited.scan_with(&mut scanner)?;
        let (key, value) = scanner.into_scan_output();
        map.insert(key, value);
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<
    K: MapKey + ?Sized,
    V: ?Sized,
    SV: OnScanField<R> + IntoResettableScanner<Resettable: IntoScanOutput>,
    R: ReadTypes,
> IntoResettableScanner for SaveMapScanner<K, V, SV, R>
where
    Save: IntoScanner<K, Scanner<R>: OnScanField<R> + IntoScanOutput<ScanOutput: Hash + Eq>>,
{
    type Resettable = SaveMapScanner<K, V, SV::Resettable, R>;
    fn into_resettable(self) -> Self::Resettable {
        let Self(_map, sv, PhantomData, PhantomData) = self;
        SaveMapScanner(
            HashMap::new(),
            sv.into_resettable(),
            PhantomData,
            PhantomData,
        )
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanOutput, R: ReadTypes> ResettableScanner
    for SaveMapScanner<K, V, SV, R>
where
    Save: IntoScanner<K>,
{
    fn reset(&mut self) {
        self.0.clear();
    }
}
