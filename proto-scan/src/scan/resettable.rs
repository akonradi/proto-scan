pub trait Resettable {
    type Mark: Clone;
    fn mark(&mut self) -> Self::Mark;

    fn reset(&mut self, to: Self::Mark);
}

/// A type that can be transformed into a [`Resettable`] type.
/// 
/// Implementations can use `Self` for [`IntoResettable::Resettable`] if they
/// already implement the trait.
pub trait IntoResettable {
    type Resettable: Resettable;

    fn into_resettable(self) -> Self::Resettable;
}

impl<T> Resettable for &mut Vec<T> {
    type Mark = usize;

    fn mark(&mut self) -> Self::Mark {
        self.len()
    }

    fn reset(&mut self, to: Self::Mark) {
        self.truncate(to);
    }
}
