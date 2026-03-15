use core::convert::Infallible;
use core::hash::Hash;
use core::marker::PhantomData;
#[cfg(any(feature = "std", doc))]
use std::collections::HashMap;

use either::Either;

use crate::read::{ReadBuffer, ReadTypes};
use crate::scan::IntoScanner;
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::{OnScanField, SaveBytesScanner};
use crate::scan::{IntoScanOutput, ScanCallbacks, ScanError};
use crate::wire::{GroupOp, LengthDelimited, NumericField};

/// Marker type for protobuf map field
pub struct Map<K: ?Sized, V: ?Sized>(PhantomData<K>, PhantomData<V>, Infallible);

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

/// Valid key type for a map.
/// 
/// Per the protobuf documentation this can be any integral or string type. This
/// is implemented on the type representing the protobuf wire format, like
/// [`Varint`] or [`str`].
pub trait MapKey {
    /// Rust type for the map key.
    type Repr<R: ReadTypes>: Default + Hash + Eq;

    /// Scanner for the map key.
    // This should just be from a IntoScanner supertrait but there's no way to
    // specify a "forall" bound on the Scanner generic associated type.
    type Scanner<R: ReadTypes>: OnScanField<R>
        + IntoScanOutput<ScanOutput = Option<Self::ScannerOutput<R>>>
        + Default;

    /// Output type of the map key scanner.
    /// 
    /// Ideally this would be an `impl Into<Self::Output<R>>` in place of its
    /// usage in the [`Self::Scanner`] bound above but the language doesn't
    /// allow that.
    type ScannerOutput<R: ReadTypes>: Into<Self::Repr<R>>;
}

macro_rules! impl_map_key_for {
    ($p:ty) => {
        impl MapKey for $p
        where
            <Self as Encoding>::Repr: Default + Hash + Eq,
        {
            type Repr<R: ReadTypes> = <Self as Encoding>::Repr;
            type Scanner<R: ReadTypes> = super::field::SaveNumeric<Self>;
            type ScannerOutput<R: ReadTypes> = Self::Repr<R>;
        }
    };
}

impl_map_key_for!(Varint<bool>);
impl_map_key_for!(Varint<i32>);
impl_map_key_for!(Varint<i64>);
impl_map_key_for!(Varint<u32>);
impl_map_key_for!(Varint<u64>);
impl_map_key_for!(Varint<ZigZag<i32>>);
impl_map_key_for!(Varint<ZigZag<i64>>);
impl_map_key_for!(Fixed<u64>);
impl_map_key_for!(Fixed<u32>);
impl_map_key_for!(Fixed<i64>);
impl_map_key_for!(Fixed<i32>);

impl MapKey for str {
    type Repr<R: ReadTypes> = <R::Buffer as ReadBuffer>::String;

    type ScannerOutput<R: ReadTypes> = <R::Buffer as ReadBuffer>::String;
    type Scanner<R: ReadTypes> = SaveBytesScanner<str, R>;
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized, SV: IntoScanner<V>> IntoScanner<Map<K, V>> for SaveMap<SV> {
    type Scanner<R: ReadTypes> = SaveMapScanner<K, SV::Scanner<R>, R>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveMapScanner(HashMap::new(), self.0.into_scanner(), PhantomData)
    }
}

#[cfg(feature = "std")]
pub struct SaveMapScanner<K: MapKey + ?Sized, SV: IntoScanOutput, R: ReadTypes>(
    HashMap<K::Repr<R>, SV::ScanOutput>,
    SV,
    PhantomData<R>,
);

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, SV: IntoScanOutput, R: ReadTypes> IntoScanOutput
    for SaveMapScanner<K, SV, R>
{
    type ScanOutput = HashMap<K::Repr<R>, SV::ScanOutput>;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, SV: OnScanField<R> + Clone, R: ReadTypes> OnScanField<R>
    for SaveMapScanner<K, SV, R>
{
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_group(&mut self, _op: GroupOp) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        Err(ScanError::WrongWireType)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<<R>::Error>> {
        let Self(map, value_scanner, PhantomData) = self;
        let scanner = MapEntry(K::Scanner::default(), value_scanner.clone());
        let scan = crate::scan::Scan::new(delimited.into_events(), scanner);
        let (key, value) = scan.read_all()?;
        let key = key.map(Into::into).unwrap_or_default();
        map.insert(key, value);
        Ok(None)
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
        op: GroupOp,
    ) -> Result<Self::ScanEvent, ScanError<<R>::Error>> {
        Ok(match u32::from(field) {
            1 => self.0.on_group(op)?.map(Either::Left),
            2 => self.1.on_group(op)?.map(Either::Right),
            _ => None,
        })
    }

    fn on_length_delimited(
        &mut self,
        field: crate::wire::FieldNumber,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Self::ScanEvent, ScanError<<R>::Error>> {
        Ok(match u32::from(field) {
            1 => self.0.on_length_delimited(delimited)?.map(Either::Left),
            2 => self.1.on_length_delimited(delimited)?.map(Either::Right),
            _ => None,
        })
    }
}
