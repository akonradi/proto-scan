use core::convert::Infallible;

use crate::read::ReadTypes;
use crate::scan::field::OnScanField;
use crate::scan::{
    IntoResettable, IntoScanOutput, IntoScanner, Resettable, ScanCallbacks, StopScan, next_event,
};
use crate::wire::LengthDelimited;

pub struct Message<F>(F);

impl<F: Resettable> Message<F> {
    pub fn new(scanner: impl IntoResettable<Resettable = F>) -> Self {
        Self(scanner.into_resettable())
    }
}

impl<F: ScanCallbacks<R, ScanOutput: Default> + Resettable, R: ReadTypes> OnScanField<R>
    for Message<F>
{
    type ScanEvent = Infallible;

    fn on_numeric(
        &mut self,
        _value: crate::scan::NumericField,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: crate::scan::GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited<ReadTypes = R>,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        self.0.reset();
        let mut parse = delimited.into_events();
        let fields = &mut self.0;
        while let Some(next) = next_event(&mut parse, fields) {
            let _event: F::ScanEvent = next?;
        }
        Ok(None)
    }
}

impl<F: IntoScanOutput<ScanOutput: Default>> IntoScanOutput for Message<F> {
    type ScanOutput = F::ScanOutput;

    fn into_scan_output(self) -> Self::ScanOutput {
        self.0.into_scan_output()
    }
}

impl<F: Resettable> Resettable for Message<F> {
    fn reset(&mut self) {
        self.0.reset()
    }
}

impl<F: IntoScanner> IntoScanner for Message<F> {
    type Scanner<R: ReadTypes> = Message<F::Scanner<R>>;
    fn into_scanner<R: ReadTypes>(self) -> Self::Scanner<R> {
        Message(self.0.into_scanner())
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;
    use hex_literal::hex;

    use crate::scan::encoding::Varint;
    use crate::scan::field::{NoOp, SaveNumeric};
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
        ) -> Result<Self::ScanEvent, StopScan> {
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
        ) -> Result<Self::ScanEvent, StopScan> {
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
        ) -> Result<Self::ScanEvent, StopScan> {
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

    impl<T: Resettable> Resettable for Scanner<T> {
        fn reset(&mut self) {
            self.1.reset();
        }
    }

    impl<T: IntoResettable> IntoResettable for Scanner<T> {
        type Resettable = Scanner<T::Resettable>;
        fn into_resettable(self) -> Self::Resettable {
            let Self(f, t) = self;
            Scanner(f, t.into_resettable())
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
            Message::new(Scanner(1, SaveNumeric::<Varint<i32>>::new())),
        );

        let mut input = &INPUT[..];
        let scan = Scan::new(crate::wire::parse(&mut input), scanner);
        let result = scan.read_all();

        assert_matches!(result, Ok(ScanOutput(ScanOutput(Some(150)))))
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

        // Duplicating the input should not result in multiple outputs written
        // to saved_to. That mirrors protobuf's last-one-wins semantics.
        for input in [Vec::from(INPUT), INPUT.repeat(2)] {
            let mut saved_to = vec![1, 2, 3];
            let scanner = Scanner(
                3,
                Message::new(Scanner(
                    1,
                    crate::scan::field::WriteRepeated::<Varint<i32>, _>::new(&mut saved_to),
                )),
            );

            let mut input = &input[..];
            let scan = Scan::new(crate::wire::parse(&mut input), scanner);
            let result = scan.read_all();

            assert_matches!(result, Ok(ScanOutput(ScanOutput(()))));
            assert_eq!(saved_to, &[1, 2, 3, 150, 151]);
        }
    }
}
