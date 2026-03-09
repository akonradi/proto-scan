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
            int32_field: None,
            int64_field: None,
            uint32_field: None,
            uint64_field: None,
            sint32_field: None,
            sint64_field: None,
            bool_field: None,
            enum_field: (),
            fixed64_field: None,
            sfixed64_field: None,
            double_field: None,
            fixed32_field: None,
            sfixed32_field: None,
            float_field: None,
        };
        assert_eq!(output, expected);
    }
}
