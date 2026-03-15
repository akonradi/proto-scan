/// A scanner that can be "reset" after handling some events.
///
/// This is used to implement protobuf oneof semantics. When one of the fields
/// in a oneof is encountered after a different field was encountered earlier
/// during parsing, the scanner for the earlier field has its
/// [`ResettableScanner::reset`] method called to signal the invalidation.
pub trait ResettableScanner {
    fn reset(&mut self);
}

/// A type that can be transformed into a [`ResettableScanner`] type.
///
/// Implementations can use `Self` for [`IntoResettableScanner::Resettable`] if they
/// already implement the trait.
pub trait IntoResettableScanner {
    type Resettable: ResettableScanner;

    fn into_resettable(self) -> Self::Resettable;
}
