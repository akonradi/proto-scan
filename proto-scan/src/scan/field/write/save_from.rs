/// Generalization of assignment for wrapping types.
///
/// A type `Wrap<T>(T, bool)` could implement `SaveFrom<T>` to save the value
/// and note that a modification was made.
pub trait SaveFrom<T> {
    fn save_from(&mut self, value: T);
}

impl<S, T: From<S>> SaveFrom<S> for &mut T {
    fn save_from(&mut self, value: S) {
        **self = value.into();
    }
}
