use std::convert::Infallible;
use std::marker::PhantomData;

use crate::read::ReadTypes;
use crate::scan::field::{NoOutput, OnScanField};
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, MessageScanner, ResettableScanner, ScanCallbacks,
    ScanError, ScanLengthDelimited,
};
use crate::wire::WrongWireType;

/// Marker type for a message field.
#[derive(Clone)]
pub struct Message<F>(PhantomData<F>, Infallible);

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Option<Message<M>>> for S {
    type Scanner<R: ReadTypes> = Scanner<S::Scanner<R>, Option<Present>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Scanner {
            scanner: S::into_scanner(self),
            presence: None,
        }
    }
}

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Message<M>> for S {
    type Scanner<R: ReadTypes> = Scanner<S::Scanner<R>, Present>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Scanner {
            scanner: S::into_scanner(self),
            presence: Present,
        }
    }
}

/// [`IntoScanner`] wrapper for an embedded message field that should be treated
/// as empty if not present.
#[derive(Clone, Debug)]
pub struct EmptyIfMissing<S>(S);

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Option<Message<M>>>
    for EmptyIfMissing<S>
{
    type Scanner<R: ReadTypes> = Scanner<S::Scanner<R>, Present>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Scanner {
            scanner: S::into_scanner(self.0),
            presence: Present,
        }
    }
}

/// Extension trait for [`MessageScanner`] for scanning embedded message fields.
///
/// This trait is blanket-implemented for all `MessageScanner`s.
pub trait ScanOptionalMessage: MessageScanner + Sized {
    /// Transforms a message scanner into one that treats a message field as empty if it is not seen.
    ///
    /// Embedded message fields in protobuf messages are implicitly optional.
    /// That means that the wire format (and serializer and deserializer APIs)
    /// differentiate between an embedded message field that is not present and
    /// one present with an empty embedded message value.
    ///
    /// Embedded message fields require a [`IntoScanner<Option<Message<M>>>`].
    /// The blanket impl for [`MessageScanner`]s tracks whether the message was
    /// seen at all, which requires additional overhead. In return, it produces
    /// as its [`IntoScanOutput::ScanOutput`] an `Option`.
    ///
    /// In cases where the fidelity (and overhead) are not needed, this method
    /// can be used to avoid it. The [`EmptyIfMissing`] wrapper retured by this
    /// method implements `IntoScanner<Option<Message<M>>>` by assuming the
    /// message was seen at least once. This lets its
    /// `IntoScanOutput::ScanOutput` be non-optional.
    fn empty_if_not_present(self) -> EmptyIfMissing<Self> {
        EmptyIfMissing(self)
    }
}
impl<M: MessageScanner + Sized> ScanOptionalMessage for M {}

#[derive(Debug, Clone)]
pub struct Scanner<S, P> {
    scanner: S,
    presence: P,
}

#[derive(Copy, Clone, Default, PartialEq)]
pub struct Present;

pub trait Presence: PartialEq<Present> + From<Present> + Default {
    type Output<T>;

    fn then<R>(self, f: impl FnOnce() -> R) -> Self::Output<R>;
}

impl<F: ScanCallbacks<R> + IntoScanOutput, R: ReadTypes, P: Presence> OnScanField<R>
    for Scanner<F, P>
{
    fn on_numeric(&mut self, _value: crate::scan::NumericField) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        delimited: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        let Self { scanner, presence } = self;
        delimited.scan_with(NoOutput(scanner))?;
        *presence = Present.into();
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        let Self { scanner, presence } = self;
        delimited.scan_with(NoOutput(scanner))?;
        *presence = Present.into();
        Ok(())
    }
}

impl<F: IntoScanOutput, P: Presence> IntoScanOutput for Scanner<F, P> {
    type ScanOutput = P::Output<F::ScanOutput>;

    fn into_scan_output(self) -> Self::ScanOutput {
        let Self { presence, scanner } = self;
        presence.then(|| scanner.into_scan_output())
    }
}

impl<F: ResettableScanner, P: Presence> ResettableScanner for Scanner<F, P> {
    fn reset(&mut self) {
        let Self { presence, scanner } = self;
        *presence = P::default();
        scanner.reset();
    }
}

impl PartialEq<Present> for Option<Present> {
    fn eq(&self, _: &Present) -> bool {
        match self {
            None => false,
            Some(Present) => true,
        }
    }
}

impl Presence for Option<Present> {
    type Output<T> = Option<T>;
    fn then<R>(self, f: impl FnOnce() -> R) -> Self::Output<R> {
        self.map(|Present| f())
    }
}

impl Presence for Present {
    type Output<T> = T;
    fn then<R>(self, f: impl FnOnce() -> R) -> Self::Output<R> {
        f()
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;
    use hex_literal::hex;

    use crate::scan::encoding::Varint;
    use crate::scan::field::{NoOp, Save};
    #[cfg(feature = "std")]
    use crate::scan::field::{Repeated, Write};
    use crate::scan::{FieldNumber, GroupDelimited, NumericField, Scan};

    use super::*;
    struct Scanner<T = NoOp>(u32, T);
    #[derive(Debug, Default, PartialEq)]
    struct ScanOutput<T>(T);

    impl<R: ReadTypes, T: OnScanField<R>> ScanCallbacks<R> for Scanner<T> {
        fn on_numeric(
            &mut self,
            field: FieldNumber,
            value: NumericField,
        ) -> Result<(), ScanError<R::Error>> {
            if field == self.0 {
                self.1.on_numeric(value)
            } else {
                Ok(())
            }
        }

        fn on_group(
            &mut self,
            field: FieldNumber,
            delimited: impl GroupDelimited<ReadTypes = R>,
        ) -> Result<(), ScanError<<R>::Error>> {
            if field == self.0 {
                self.1.on_group(delimited)
            } else {
                Ok(())
            }
        }

        fn on_length_delimited(
            &mut self,
            field: FieldNumber,
            delimited: impl ScanLengthDelimited<ReadTypes = R>,
        ) -> Result<(), ScanError<R::Error>> {
            if field == self.0 {
                self.1.on_length_delimited(delimited)
            } else {
                Ok(())
            }
        }
    }

    impl<T: IntoScanOutput> IntoScanOutput for Scanner<T> {
        type ScanOutput = ScanOutput<T::ScanOutput>;
        fn into_scan_output(self) -> Self::ScanOutput {
            ScanOutput(self.1.into_scan_output())
        }
    }

    #[test]
    fn scan_embedded_message() {
        /// ```proto
        /// message Test1 {
        ///   int32 a = 1;
        /// }
        ///
        /// message Test3 {
        ///   Test1 c = 3;
        /// }
        /// ```
        /// Test1’s a field (i.e., Test3’s c.a field) is set to 150
        const INPUT: &[u8] = &hex!("1a 03 08 96 01");

        let scanner = Scanner(
            3,
            super::Scanner {
                scanner: Scanner(
                    1,
                    <Save as IntoScanner<Varint<i32>>>::into_scanner::<&[u8]>(Save),
                ),
                presence: Present,
            },
        );

        let mut input = &INPUT[..];
        let scan = Scan::new(crate::wire::parse(&mut input), scanner);
        let result = scan.read_all();

        assert_matches!(result, Ok(ScanOutput(ScanOutput(150))))
    }

    #[test]
    #[cfg(feature = "std")]
    fn save_from_embedded_repeated() {
        /// ```proto
        /// message Test1 {
        ///   repeated int32 a = 1;
        /// }
        ///
        /// message Test3 {
        ///   Test1 c = 3;
        /// }
        /// ```
        /// Test1’s a field (i.e., Test3’s c.a field) is set to [150, 151]
        const INPUT: &[u8] = &hex!("1a 06 08 96 01 08 97 01");

        // Duplicating the input should result in multiple outputs written
        // to saved_to. That mirrors protobuf's merge semantics for embedded messages.
        for repeats in [1, 2] {
            let input = INPUT.repeat(repeats);
            let mut saved_to = vec![1, 2, 3];
            let scanner = Scanner(
                3,
                super::Scanner {
                    scanner: (Scanner(
                        1,
                        <Write<_> as IntoScanner<Repeated<Varint<i32>>>>::into_scanner::<&[u8]>(
                            Write(&mut saved_to),
                        ),
                    )),
                    presence: Present,
                },
            );

            let mut input = &input[..];
            let scan = Scan::new(crate::wire::parse(&mut input), scanner);
            let result = scan.read_all();

            assert_matches!(result, Ok(ScanOutput(ScanOutput(()))));
            assert_eq!(saved_to[..3], [1, 2, 3]);
            assert_eq!(saved_to[3..], [150, 151].repeat(repeats))
        }
    }
}
