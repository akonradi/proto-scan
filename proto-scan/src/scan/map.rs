use core::convert::Infallible;
use core::marker::PhantomData;

/// Marker type for protobuf map field
pub struct Map<K: ?Sized, V: ?Sized>(PhantomData<K>, PhantomData<V>, Infallible);
