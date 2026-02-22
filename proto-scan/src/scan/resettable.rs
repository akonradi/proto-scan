pub trait Resettable {
    type Mark: Clone;
    fn mark(&mut self) -> Self::Mark;

    fn reset(&mut self, to: Self::Mark);
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
