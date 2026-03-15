#![no_std]
use proto_scan_tests::proto;

mod cases {
    use proto_scan::scan::field::Save;
    use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};

    use super::*;
    #[test]
    fn save_numeric_fields() {
        let scanner = proto::SingleFieldTypes::scanner()
            .int32_field(Save)
            .int64_field(Save)
            .uint32_field(Save)
            .uint64_field(Save)
            .sint32_field(Save)
            .sint64_field(Save)
            .bool_field(Save)
            .fixed64_field(Save)
            .sfixed64_field(Save)
            .double_field(Save)
            .fixed32_field(Save)
            .sfixed32_field(Save)
            .float_field(Save);

        let output = {
            let bytes = [];
            scanner.scan(bytes.as_slice()).read_all().unwrap()
        };

        let expected = proto::ScanSingleFieldTypesOutput {
            int32_field: 0,
            int64_field: 0,
            uint32_field: 0,
            uint64_field: 0,
            sint32_field: 0,
            sint64_field: 0,
            bool_field: false,
            enum_field: (),
            fixed64_field: 0,
            sfixed64_field: 0,
            double_field: 0.0,
            fixed32_field: 0,
            sfixed32_field: 0,
            float_field: 0.0,
        };
        assert_eq!(output, expected);
    }
}
