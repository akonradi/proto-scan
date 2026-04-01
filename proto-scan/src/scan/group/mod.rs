mod delimited;
pub use delimited::GroupDelimited;
pub(super) use delimited::GroupDelimitedImpl;
mod error;
mod stack;
pub use stack::GroupStack;

#[cfg(test)]
mod test {
    use arrayvec::ArrayVec;
    use assert_matches::assert_matches;

    use crate::read::ReadTypes;
    use crate::scan::{IntoScanOutput, Scan, ScanCallbacks, ScanError, ScanLengthDelimited};
    use crate::wire::{FieldNumber, NumericField, Tag, WireType, parse, serialize_base128_varint};

    use super::*;

    struct ScannerImpl {
        field_2: ArrayVec<u64, 2>,
    }

    impl<R: ReadTypes> ScanCallbacks<R> for ScannerImpl {
        fn on_numeric(
            &mut self,
            field: FieldNumber,
            value: crate::wire::NumericField,
        ) -> Result<(), ScanError<<R>::Error>> {
            if field == 2 {
                let value = assert_matches!(value, NumericField::Varint(v) => v);
                self.field_2.push(value);
            }
            Ok(())
        }
        fn on_group(
            &mut self,
            _field: FieldNumber,
            _group: impl GroupDelimited<ReadTypes = R>,
        ) -> Result<(), ScanError<<R>::Error>> {
            Ok(())
        }

        fn on_length_delimited(
            &mut self,
            _field: FieldNumber,
            _delimited: impl ScanLengthDelimited<ReadTypes = R>,
        ) -> Result<(), ScanError<<R>::Error>> {
            Ok(())
        }
    }

    impl IntoScanOutput for ScannerImpl {
        type ScanOutput = ArrayVec<u64, 2>;
        fn into_scan_output(self) -> Self::ScanOutput {
            self.field_2
        }
    }

    #[test]
    fn scan_ignored_group() {
        let input = [
            Tag {
                field_number: FieldNumber::new(2).unwrap(),
                wire_type: WireType::Varint,
            }
            .serialized(),
            serialize_base128_varint(22u32),
            Tag {
                field_number: FieldNumber::new(3).unwrap(),
                wire_type: WireType::Sgroup,
            }
            .serialized(),
            Tag {
                field_number: FieldNumber::new(5).unwrap(),
                wire_type: WireType::Varint,
            }
            .serialized(),
            serialize_base128_varint(33u32),
            Tag {
                field_number: FieldNumber::new(3).unwrap(),
                wire_type: WireType::Egroup,
            }
            .serialized(),
            Tag {
                field_number: FieldNumber::new(2).unwrap(),
                wire_type: WireType::Varint,
            }
            .serialized(),
            serialize_base128_varint(44u32),
        ]
        .into_iter()
        .flatten()
        .collect::<ArrayVec<u8, 64>>();

        // With no group stack, an error is returned.
        let result = Scan::new(
            parse(input.as_slice()),
            ScannerImpl {
                field_2: ArrayVec::new(),
            },
        )
        .read_all();
        assert_eq!(result, Err(ScanError::GroupOverflow));

        // With space for a group, the values can be saved.
        let result = Scan::new(
            parse(input.as_slice()),
            ScannerImpl {
                field_2: ArrayVec::new(),
            },
        )
        .with_group_stack(ArrayVec::<_, 1>::new())
        .read_all();
        assert_eq!(result, Ok([22, 44].into()))
    }
}
