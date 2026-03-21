#![doc(hidden)]

#[cfg(feature = "std")]
use core::ops::DerefMut;

use crate::scan::ResettableScanner;
use crate::scan::field::write::SaveFrom;

/// Implements [`SaveFrom`] and [`Resettable`] to save and restore a previous value.
pub struct RestoreOnReset<'t, D>(&'t mut D, Option<D>);

impl<'t, D> RestoreOnReset<'t, D> {
    pub(super) fn new(destination: &'t mut D) -> Self {
        Self(destination, None)
    }
}

impl<'t, D> ResettableScanner for RestoreOnReset<'t, D> {
    fn reset(&mut self) {
        if let Some(prev) = self.1.take() {
            *self.0 = prev;
        }
    }
}

impl<'t, T: Into<D>, D> SaveFrom<T> for RestoreOnReset<'t, D> {
    fn save_from(&mut self, value: T) {
        self.1 = Some(core::mem::replace(self.0, value.into()));
    }
}

#[cfg(feature = "std")]
pub struct RestoreLenOnReset<'t, T>(&'t mut T, usize);

#[cfg(feature = "std")]
impl<'t, T> RestoreLenOnReset<'t, Vec<T>> {
    pub fn new(arg: &'t mut Vec<T>) -> Self {
        let len = arg.len();
        Self(arg, len)
    }
}

#[cfg(feature = "std")]
impl<'t, T> ResettableScanner for RestoreLenOnReset<'t, Vec<T>> {
    fn reset(&mut self) {
        self.0.truncate(self.1);
    }
}

#[cfg(feature = "std")]
impl<'t, T> std::ops::Deref for RestoreLenOnReset<'t, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(feature = "std")]
impl<'t, T> DerefMut for RestoreLenOnReset<'t, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
