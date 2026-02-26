/// A type that can be "reset" to an intial value.
pub trait Resettable {
    fn reset(&mut self);
}

/// A type that can be transformed into a [`Resettable`] type.
///
/// Implementations can use `Self` for [`IntoResettable::Resettable`] if they
/// already implement the trait.
pub trait IntoResettable {
    type Resettable: Resettable;

    fn into_resettable(self) -> Self::Resettable;
}
