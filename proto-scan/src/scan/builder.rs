use crate::read::Read;
use crate::scan::{IntoScanner, MessageScanner, Scan, ScanCallbacks};
use crate::wire::ParseEventReader;

/// A builder type for a [`Scan`] over a byte stream.
///
/// This is a convenience trait that provides `scan` and `scan_event` functions.
/// It is blanket=implemented for all [`MessageScanner`] types.
pub trait ScannerBuilder<M: ?Sized>: Sized {
    /// Starts a scan over the provided input.
    ///
    /// Consumes `self` and produces a [`Scan`] over the input stream.
    fn scan_events<P: ParseEventReader>(self, read: P) -> Scan<P, Self::Scanner<P::ReadTypes>>
    where
        Self: IntoScanner<M>,
        Self::Scanner<P::ReadTypes>: ScanCallbacks<P::ReadTypes>,
    {
        Scan::new(read, self.into_scanner())
    }

    /// Starts a scan over the provided input.
    ///
    /// Consumes `self` and produces a [`Scan`] over the input stream.
    fn scan<'r, R: Read + 'r>(
        self,
        read: R,
    ) -> Scan<impl ParseEventReader<ReadTypes = R::ReadTypes> + 'r, Self::Scanner<R::ReadTypes>>
    where
        Self: IntoScanner<M>,
        Self::Scanner<R::ReadTypes>: ScanCallbacks<R::ReadTypes>,
    {
        self.scan_events(crate::wire::parse(read))
    }
}

impl<S: MessageScanner> ScannerBuilder<S::Message> for S {}
