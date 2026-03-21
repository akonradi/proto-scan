use crate::read::{BoundsOnlyReadTypes, ReadTypes};
use crate::scan::IntoScanner;
use crate::scan::encoding::{Encoding, Fixed, Varint, ZigZag};
use crate::scan::field::{OnScanField, Repeated};

mod bytes;
use bytes::{WriteBytes, WriteRepeatedBytes};
mod numeric;
use numeric::{WriteNumeric, WriteRepeatedNumeric};
mod restore;
use restore::*;
mod save_from;
pub use save_from::SaveFrom;

pub struct Write<T>(pub T);

macro_rules! impl_into_scanner {
    ($p:path) => {
        impl<'t, T> IntoScanner<$p> for Write<&'t mut T>
        where
            <$p as Encoding>::Repr: Into<T>,
        {
            type Scanner<R: ReadTypes> = WriteNumeric<$p, &'t mut T>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                WriteNumeric::new(self.0)
            }
        }

        impl<D> IntoScanner<Repeated<$p>> for Write<D>
        where
            WriteRepeatedNumeric<$p, D>: OnScanField<BoundsOnlyReadTypes>,
        {
            type Scanner<R: ReadTypes> = WriteRepeatedNumeric<$p, D>;

            fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
                WriteRepeatedNumeric::new(self.0)
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

impl<T> IntoScanner<[u8]> for Write<T> {
    type Scanner<R: ReadTypes> = WriteBytes<[u8], T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteBytes::new(self.0)
    }
}
impl<T> IntoScanner<str> for Write<T> {
    type Scanner<R: ReadTypes> = WriteBytes<str, T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteBytes::new(self.0)
    }
}

impl<T> IntoScanner<Repeated<[u8]>> for Write<T> {
    type Scanner<R: ReadTypes> = WriteRepeatedBytes<[u8], T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteRepeatedBytes::new(self.0)
    }
}
impl<T> IntoScanner<Repeated<str>> for Write<T> {
    type Scanner<R: ReadTypes> = WriteRepeatedBytes<str, T>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        WriteRepeatedBytes::new(self.0)
    }
}

#[cfg(test)]
mod test {
    use crate::scan::field::test::assert_impl_into_scanner;

    use super::*;

    macro_rules! assert_impl_into_repeatable_scanner {
        (Write<&mut $t:ty>: IntoScanner<$p:ty>) => {
            assert_impl_into_scanner!(Write<&mut $t>: IntoScanner<$p>; resettable);
            #[cfg(feature = "std")]
            assert_impl_into_scanner!(Write<&mut Vec<$t>>: IntoScanner<Repeated<$p>>; resettable);
        };
    }

    assert_impl_into_repeatable_scanner!(Write<&mut i32>: IntoScanner<Varint<i32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut bool>: IntoScanner<Varint<bool>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i32>: IntoScanner<Varint<i32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i64>: IntoScanner<Varint<i64>>);
    assert_impl_into_repeatable_scanner!(Write<&mut u32>: IntoScanner<Varint<u32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut u64>: IntoScanner<Varint<u64>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i32>: IntoScanner<Varint<ZigZag<i32>>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i64>: IntoScanner<Varint<ZigZag<i64>>>);
    assert_impl_into_repeatable_scanner!(Write<&mut u64>: IntoScanner<Fixed<u64>>);
    assert_impl_into_repeatable_scanner!(Write<&mut u32>: IntoScanner<Fixed<u32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i64>: IntoScanner<Fixed<i64>>);
    assert_impl_into_repeatable_scanner!(Write<&mut i32>: IntoScanner<Fixed<i32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut f64>: IntoScanner<Fixed<f64>>);
    assert_impl_into_repeatable_scanner!(Write<&mut f32>: IntoScanner<Fixed<f32>>);
    assert_impl_into_repeatable_scanner!(Write<&mut &'static str>: IntoScanner<str>);
    assert_impl_into_repeatable_scanner!(Write<&mut [u8; 0]>: IntoScanner<[u8]>);
}
