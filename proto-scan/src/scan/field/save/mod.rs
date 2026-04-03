//! Provides [`IntoScanner`] and [`OnScanField`](super::OnScanField) types that
//! implement conventional protobuf behavior.
//!
//! The [`Save`] unit type is the entry point for this module. It can be passed
//! as an `IntoScanner<T>` impl for all scalar types `T`.
use crate::read::ReadTypes;
use crate::scan::IntoScanner;
use crate::scan::encoding::{Fixed, Varint, ZigZag};
#[cfg(feature = "std")]
use crate::scan::field::save::map::SaveMapScanner;
#[cfg(feature = "std")]
use crate::scan::field::save::repeated::SaveRepeatedBytes;
#[cfg(feature = "std")]
use crate::scan::field::{Map, MapKey, Repeated};

mod bytes;
pub(crate) use bytes::DecodeFromBytes;
use bytes::SaveBytesScanner;
mod cast;
mod map;
use map::SaveMap;
mod optional;
use optional::SaveOptional;
mod numeric;
use numeric::SaveNumeric;
mod repeated;
pub use repeated::SaveCloned;
#[cfg(feature = "std")]
use repeated::SaveRepeated;

/// [`IntoScanner`] implementation that produces the read value as the event output.
///
/// The [`IntoScanner::Scanner`] types provided by `Save` implement standard
/// protobuf message semantics (last-one wins for scalar and oneof fields,
/// merging messages). See the [`scan`](super::super) documentation for usage
/// examples.
#[derive(Clone)]
pub struct Save;

impl Save {
    /// Returns an [`IntoScanner`] for a map field.
    ///
    /// Produces a type that implements `IntoScanner` by saving each key and the
    /// result of scanning the value with the provided scanner.
    pub fn with_value<V>(value_scanner: V) -> SaveMap<V> {
        SaveMap::with_value(value_scanner)
    }
}

macro_rules! impl_into_scanner {
    ($p:path) => {
        impl IntoScanner<$p> for Save {
            type Scanner<R: ReadTypes> = SaveNumeric<$p>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                SaveNumeric::default()
            }
        }
        #[cfg(feature = "std")]
        impl IntoScanner<super::Repeated<$p>> for Save {
            type Scanner<R: ReadTypes> = SaveRepeated<$p>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                SaveRepeated::default()
            }
        }
    };
}

impl_into_scanner!(Varint<bool>);
impl_into_scanner!(Varint<i32>);
impl_into_scanner!(Varint<i64>);
impl_into_scanner!(Varint<u32>);
impl_into_scanner!(Varint<u64>);
impl_into_scanner!(Varint<ZigZag<i32>>);
impl_into_scanner!(Varint<ZigZag<i64>>);
impl_into_scanner!(Fixed<u64>);
impl_into_scanner!(Fixed<u32>);
impl_into_scanner!(Fixed<i64>);
impl_into_scanner!(Fixed<i32>);
impl_into_scanner!(Fixed<f64>);
impl_into_scanner!(Fixed<f32>);

impl IntoScanner<str> for Save {
    type Scanner<R: ReadTypes> = SaveBytesScanner<str, R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner::new()
    }
}

impl IntoScanner<[u8]> for Save {
    type Scanner<R: ReadTypes> = SaveBytesScanner<[u8], R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveBytesScanner::new()
    }
}

impl<N> IntoScanner<Option<N>> for Save
where
    Save: IntoScanner<N>,
{
    type Scanner<R: ReadTypes> = SaveOptional<<Save as IntoScanner<N>>::Scanner<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveOptional {
            inner: Save.into_scanner(),
            present: false,
        }
    }
}

#[cfg(feature = "std")]
impl IntoScanner<Repeated<str>> for Save {
    type Scanner<R: ReadTypes> = SaveRepeatedBytes<str, R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveRepeatedBytes::new()
    }
}

#[cfg(feature = "std")]
impl IntoScanner<Repeated<[u8]>> for Save {
    type Scanner<R: ReadTypes> = SaveRepeatedBytes<[u8], R>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        SaveRepeatedBytes::new()
    }
}

#[cfg(feature = "std")]
impl<K: MapKey + ?Sized, V: ?Sized> IntoScanner<Map<K, V>> for Save
where
    Save: IntoScanner<K>,
    Save: IntoScanner<V>,
{
    type Scanner<R: ReadTypes> = SaveMapScanner<K, V, R, Save>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        IntoScanner::<Map<K, V>>::into_scanner(Save::with_value(Save))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::scan::field::test::assert_impl_into_scanner;

    macro_rules! assert_impl_into_resettable_scanner {
        ($t:ty: IntoScanner<$p:ty>) => {
            assert_impl_into_scanner!($t: IntoScanner<$p>; resettable);
            #[cfg(feature = "std")]
            assert_impl_into_scanner!($t: IntoScanner<$crate::scan::field::Repeated<$p>>; resettable);
        };
        ($t:ty: IntoScanner<$p:ty>; non-repeatable) => {
            assert_impl_into_scanner!($t: IntoScanner<$p>);
        };
    }

    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<bool>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<i32>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<i64>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<u32>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<u64>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<ZigZag<i32>>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Varint<ZigZag<i64>>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<u64>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<u32>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<i64>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<i32>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<f64>>);
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Fixed<f32>>);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<str>);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<[u8]>);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Map<str, str>>; non-repeatable);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Map<str, Fixed<f64>>>; non-repeatable);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Map<Varint<i64>, str>>; non-repeatable);
    #[cfg(feature = "std")]
    assert_impl_into_resettable_scanner!(Save: IntoScanner<Map<Varint<i64>, Fixed<f64>>>; non-repeatable);
}
