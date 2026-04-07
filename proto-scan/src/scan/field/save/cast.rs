use core::marker::PhantomData;

use crate::scan::IntoScanner;
use crate::scan::field::Save;

pub struct SaveAs<S: ?Sized, T: ?Sized>(PhantomData<S>, PhantomData<T>);

impl Save {
    pub fn as_bytes() -> SaveAs<[u8], str> {
        SaveAs(PhantomData, PhantomData)
    }
}

impl<S: ?Sized, T: ?Sized> IntoScanner<T> for SaveAs<S, T>
where
    Save: IntoScanner<S>,
{
    type Scanner<R: crate::read::ReadTypes> = <Save as IntoScanner<S>>::Scanner<R>;

    fn into_scanner<R: crate::read::ReadTypes>(self) -> Self::Scanner<R> {
        Save.into_scanner()
    }
}
