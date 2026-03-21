use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    GroupDelimited, IntoScanOutput, IntoScanner, MessageScanner, ResettableScanner, ScanCallbacks,
    ScanError, ScanLengthDelimited,
};
use crate::wire::WrongWireType;

/// Wrapper type that scans an embedded message.
#[derive(Clone)]
pub struct Message<F>(F);

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Message<M>> for S {
    type Scanner<R: ReadTypes> = Message<S::Scanner<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Message(S::into_scanner(self))
    }
}

impl<F: ScanCallbacks<R> + IntoScanOutput, R: ReadTypes> OnScanField<R> for Message<F> {
    fn on_numeric(&mut self, _value: crate::scan::NumericField) -> Result<(), ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        delimited: impl GroupDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<<R>::Error>> {
        delimited.scan_with(&mut self.0)?;
        Ok(())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl ScanLengthDelimited<ReadTypes = R>,
    ) -> Result<(), ScanError<R::Error>> {
        delimited.scan_with(&mut self.0)?;
        Ok(())
    }
}

impl<F: IntoScanOutput> IntoScanOutput for Message<F> {
    type ScanOutput = F::ScanOutput;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0.into_scan_output()
    }
}

impl<F: ResettableScanner> ResettableScanner for Message<F> {
    fn reset(&mut self) {
        self.0.reset()
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
            Message(Scanner(
                1,
                <Save as IntoScanner<Varint<i32>>>::into_scanner::<&[u8]>(Save),
            )),
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
                Message(Scanner(
                    1,
                    <Write<_> as IntoScanner<Repeated<Varint<i32>>>>::into_scanner::<&[u8]>(Write(
                        &mut saved_to,
                    )),
                )),
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
