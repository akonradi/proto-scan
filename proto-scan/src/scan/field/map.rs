use core::convert::Infallible;
use core::hash::Hash;
use core::marker::PhantomData;

use derive_where::derive_where;

use crate::read::ReadTypes;
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::repeated::RepeatedScanner;
use crate::scan::field::{Message, NoOp, OnScanField, RepeatStrategy, Repeated};
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, MessageScanner, ScanCallbacks, ScanError,
    ScanLengthDelimited, ScanMessage,
};
use crate::wire::{FieldNumber, NumericField};

/// Marker type for protobuf map field
pub struct Map<K: ?Sized, V: ?Sized>(PhantomData<K>, PhantomData<V>, Infallible);

/// Valid key type for a map.
///
/// Per the protobuf documentation this can be any integral or string type. This
/// is implemented on the type representing the protobuf wire format, like
/// [`Varint`] or [`str`].
pub trait MapKey {}

macro_rules! impl_map_key_for {
    ($p:ty) => {
        impl MapKey for $p where <Self as Encoding>::Repr: Default + Hash + Eq {}
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

impl MapKey for str {}

/// Synthetic message type for a protobuf map entry.
///
/// Protobuf maps are encoded as repeated messages where the embedded message
/// type holds the key as field 1 and the value as field 2.
pub struct MapEntry<K: ?Sized, V: ?Sized>(Infallible, PhantomData<K>, PhantomData<V>);

/// Scanner for [`MapEntry`].
#[derive_where(Clone, Debug; SK, SV)]
pub struct MapEntryScanner<K: ?Sized, V: ?Sized, SK, SV>(SK, SV, PhantomData<K>, PhantomData<V>);

impl<K: ?Sized + MapKey, V: ?Sized> ScanMessage for MapEntry<K, V> {
    type ScannerBuilder = MapEntryScanner<K, V, NoOp, NoOp>;
    fn scanner() -> Self::ScannerBuilder {
        MapEntryScanner(NoOp, NoOp, PhantomData, PhantomData)
    }
}

impl<K: MapKey + ?Sized, V: ?Sized, SK, SV> MessageScanner for MapEntryScanner<K, V, SK, SV> {
    type Message = MapEntry<K, V>;
}

impl<K: MapKey + ?Sized, V: ?Sized, SV> MapEntryScanner<K, V, NoOp, SV> {
    pub fn key<SK: IntoScanner<K>>(self, key_scanner: SK) -> MapEntryScanner<K, V, SK, SV> {
        MapEntryScanner::new(key_scanner, self.1)
    }
}

impl<K: MapKey + ?Sized, V: ?Sized, SK> MapEntryScanner<K, V, SK, NoOp> {
    pub fn value<SV: IntoScanner<V>>(self, value_scanner: SV) -> MapEntryScanner<K, V, SK, SV> {
        MapEntryScanner::new(self.0, value_scanner)
    }
}

impl<K: ?Sized, V: ?Sized, SK, SV> MapEntryScanner<K, V, SK, SV> {
    pub(crate) fn new(key: SK, value: SV) -> Self {
        Self(key, value, PhantomData, PhantomData)
    }
}

impl<K: MapKey + ?Sized, V: ?Sized, SK: IntoScanner<K>, SV: IntoScanner<V>>
    IntoScanner<MapEntry<K, V>> for MapEntryScanner<K, V, SK, SV>
{
    type Scanner<R: ReadTypes> = MapEntryScanner<K, V, SK::Scanner<R>, SV::Scanner<R>>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        MapEntryScanner::new(self.0.into_scanner(), self.1.into_scanner())
    }
}

impl<K: ?Sized, V: ?Sized, SK: IntoScanOutput, SV: IntoScanOutput> IntoScanOutput
    for MapEntryScanner<K, V, SK, SV>
{
    type ScanOutput = (SK::ScanOutput, SV::ScanOutput);
    fn into_scan_output(self) -> Self::ScanOutput {
        (self.0.into_scan_output(), self.1.into_scan_output())
    }
}

impl<K: ?Sized, V: ?Sized, SK: OnScanField<R>, SV: OnScanField<R>, R: ReadTypes> ScanCallbacks<R>
    for MapEntryScanner<K, V, SK, SV>
{
    fn on_numeric(
        &mut self,
        field: FieldNumber,
        value: NumericField,
    ) -> Result<(), ScanError<<R>::Error>> {
        let _: () = match u32::from(field) {
            1 => self.0.on_numeric(value)?,
            2 => self.1.on_numeric(value)?,
            _ => (),
        };
        Ok(())
    }

    fn on_group(
        &mut self,
        field: FieldNumber,
        group: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        let _: () = match u32::from(field) {
            1 => self.0.on_group(group)?,
            2 => self.1.on_group(group)?,
            _ => (),
        };
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        field: FieldNumber,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        let _: () = match u32::from(field) {
            1 => self.0.on_length_delimited(delimited)?,
            2 => self.1.on_length_delimited(delimited)?,
            _ => (),
        };
        Ok(())
    }
}

/// A repeated scanner for [`MapEntry`] messages can be used to scan a [`Map`].
impl<
    K: MapKey + ?Sized,
    V: ?Sized,
    RS: RepeatStrategy<S>,
    S: MessageScanner<Message = MapEntry<K, V>> + IntoScanner<MapEntry<K, V>>,
> IntoScanner<Map<K, V>> for RepeatedScanner<S, RS>
{
    type Scanner<R: ReadTypes> =
        <Self as IntoScanner<Repeated<Message<MapEntry<K, V>>>>>::Scanner<R>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        IntoScanner::<Repeated<Message<MapEntry<K, V>>>>::into_scanner(self)
    }
}
