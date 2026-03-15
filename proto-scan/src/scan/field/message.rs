use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    IntoScanOutput, IntoScanner, MessageScanner, ResettableScanner, ScanCallbacks, ScanError,
    next_event,
};
use crate::wire::{LengthDelimited, WrongWireType};

#[derive(Clone)]
pub struct Message<F>(F);

impl<F> Message<F> {
    pub fn new(scanner: F) -> Self {
        Self(scanner)
    }
}

impl<M, S: MessageScanner<Message = M> + IntoScanner<M>> IntoScanner<Message<M>> for S {
    type Scanner<R: ReadTypes> = Message<S::Scanner<R>>;

    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Message::new(S::into_scanner(self))
    }
}

impl<F: ScanCallbacks<R>, R: ReadTypes> OnScanField<R> for Message<F> {
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: crate::scan::NumericField,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_group(
        &mut self,
        _op: crate::scan::GroupOp,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        Err(WrongWireType.into())
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, ScanError<R::Error>> {
        let mut parse = delimited.into_events();
        let fields = &mut self.0;
        while let Some(next) = next_event(&mut parse, fields) {
            let _event: F::ScanEvent = next?;
        }
        Ok(None)
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
    use crate::scan::field::{NoOp, Repeated, Save, Write};
    use crate::scan::{FieldNumber, NumericField, Scan};

    use super::*;
    struct Scanner<T = NoOp>(u32, T);
    #[derive(Debug, Default, PartialEq)]
    struct ScanOutput<T>(T);

    impl<R: ReadTypes, T: OnScanField<R>> ScanCallbacks<R> for Scanner<T> {
        type ScanEvent = Option<(T::ScanEvent,)>;

        fn on_numeric(
            &mut self,
            field: FieldNumber,
            value: NumericField,
        ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
            if field == self.0 {
                self.1.on_numeric(value).map(|e| e.map(|e| (e,)))
            } else {
                Ok(None)
            }
        }

        fn on_group(
            &mut self,
            field: FieldNumber,
            op: crate::scan::GroupOp,
        ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
            if field == self.0 {
                self.1.on_group(op).map(|e| e.map(|e| (e,)))
            } else {
                Ok(None)
            }
        }

        fn on_length_delimited(
            &mut self,
            field: FieldNumber,
            delimited: impl LengthDelimited<ReadTypes = R>,
        ) -> Result<Self::ScanEvent, ScanError<R::Error>> {
            if field == self.0 {
                self.1
                    .on_length_delimited(delimited)
                    .map(|e| e.map(|e| (e,)))
            } else {
                Ok(None)
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
            Message::new(Scanner(
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
                Message::new(Scanner(
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
