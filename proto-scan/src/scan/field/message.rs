use std::convert::Infallible;

use crate::scan::field::{OnScanField, Resettable};
use crate::scan::{Scan, ScanCallbacks, ScanTypes, Scanner, StopScan, next_event};
use crate::wire::{LengthDelimited, ParseEventReader};

pub struct Message<F>(F);

impl<F: ScanCallbacks> Message<F> {
    pub fn new(scanner: F) -> Self {
        Self(scanner)
    }
}

impl<F: ScanCallbacks> ScanTypes for Message<F> {
    type ScanEvent = Infallible;
    type ScanOutput = F::ScanOutput;
}

impl<F: ScanCallbacks<ScanOutput: Default> + Into<Self::ScanOutput> + Resettable> OnScanField
    for Message<F>
{
    fn into_output(self) -> Self::ScanOutput {
        self.0.into()
    }

    fn on_scalar(
        &mut self,
        _value: crate::scan::ScalarField,
    ) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_group(&mut self, _op: crate::scan::GroupOp) -> Result<Option<Self::ScanEvent>, StopScan> {
        Err(StopScan)
    }

    fn on_length_delimited(
        &mut self,
        delimited: impl LengthDelimited,
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

impl<F: Resettable> Resettable for Message<F> {
    fn reset(&mut self) {
        self.0.reset()
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use assert_matches::assert_matches;
    use hex_literal::hex;

    use crate::scan::encoding::Varint;
    use crate::scan::field::message::test::resettable_impl::ResettableImpl;
    use crate::scan::field::{EmitScalar, NoOp, SaveRepeated};
    use crate::scan::{FieldNumber, ScalarField};

    use super::*;
    struct Scanner<T = NoOp>(T);
    #[derive(Debug, Default, PartialEq)]
    struct ScanOutput<T>(T);
    impl<T: ScanTypes> ScanTypes for Scanner<T> {
        type ScanEvent = Option<(T::ScanEvent,)>;
        type ScanOutput = ScanOutput<T::ScanOutput>;
    }
    impl<T: OnScanField> ScanCallbacks for Scanner<T> {
        fn on_scalar(
            &mut self,
            field: FieldNumber,
            value: ScalarField,
        ) -> Result<Self::ScanEvent, StopScan> {
            self.0.on_scalar(value).map(|e| e.map(|e| (e,)))
        }

        fn on_group(
            &mut self,
            field: FieldNumber,
            op: crate::scan::GroupOp,
        ) -> Result<Self::ScanEvent, StopScan> {
            self.0.on_group(op).map(|e| e.map(|e| (e,)))
        }

        fn on_length_delimited(
            &mut self,
            field: FieldNumber,
            delimited: impl LengthDelimited,
        ) -> Result<Self::ScanEvent, StopScan> {
            self.0
                .on_length_delimited(delimited)
                .map(|e| e.map(|e| (e,)))
        }
    }

    impl<T: OnScanField> From<Scanner<T>> for ScanOutput<T::ScanOutput> {
        fn from(value: Scanner<T>) -> Self {
            Self(value.0.into_output())
        }
    }

    impl<T: Resettable> Resettable for Scanner<T> {
        fn reset(&mut self) {
            self.0.reset();
        }
    }

    mod resettable_impl {
        pub(super) struct ResettableImpl<T, M>(T, M);

        impl<'v, T> ResettableImpl<&'v mut Vec<T>, usize> {
            pub fn new_vec(v: &'v mut Vec<T>) -> Self {
                let starting_size = v.len();
                Self(v, starting_size)
            }
        }

        impl<T> super::Resettable for ResettableImpl<&mut Vec<T>, usize> {
            fn reset(&mut self) {
                self.0.truncate(self.1);
            }
        }

        impl<T> Extend<T> for ResettableImpl<&mut Vec<T>, usize> {
            fn extend<It: IntoIterator<Item = T>>(&mut self, iter: It) {
                self.0.extend(iter);
            }
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

        let scanner = Scanner(Message(Scanner(EmitScalar::<Varint<i32>>::new())));

        let mut input = &INPUT[..];
        let scan = Scan::new(crate::wire::parse(&mut input), scanner);
        let result = scan.read_all();

        assert_matches!(result, Ok(ScanOutput(ScanOutput(Some(150)))))
    }

    #[test]
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
            let mut resettable = ResettableImpl::new_vec(&mut saved_to);
            let scanner = Scanner(Message(Scanner(SaveRepeated::<'_, Varint<i32>, _>::new(
                &mut resettable,
            ))));

            let mut input = &input[..];
            let scan = Scan::new(crate::wire::parse(&mut input), scanner);
            let result = scan.read_all();

            assert_matches!(result, Ok(ScanOutput(ScanOutput(()))));
            assert_eq!(saved_to, &[1, 2, 3, 150, 151]);
        }
    }
}
