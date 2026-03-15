use core::convert::Infallible;
use core::hash::Hash;
use core::marker::PhantomData;

use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};

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
