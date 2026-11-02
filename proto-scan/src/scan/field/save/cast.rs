use core::marker::PhantomData;

use crate::scan::IntoScanner;
use crate::scan::field::{Message, Save};

pub struct SaveAs<S: ?Sized, T: ?Sized>(PhantomData<S>, PhantomData<T>);

pub trait SaveAsBytes {}

impl SaveAsBytes for str {}

impl<M> SaveAsBytes for Option<Message<M>> {}

impl Save {
    /// Returns an [`IntoScanner`] for a length-delimited field that saves the
    /// contents as bytes.
    ///
    /// This is useful for avoiding UTF-8 validation on `string` field types, or
    /// treating an embedded message field as raw bytes.
    pub fn as_bytes<T: SaveAsBytes + ?Sized>() -> SaveAs<[u8], T> {
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
