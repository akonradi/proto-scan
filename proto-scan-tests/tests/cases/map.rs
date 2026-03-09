use std::collections::HashSet;

use prost::Message as _;
use proto_scan::read::ReadTypes;
use proto_scan::scan::field::{OnScanField, Save};
use proto_scan::scan::{
    IntoScanOutput, IntoScanner, SaveMap, ScanMessage as _, ScannerBuilder as _,
};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_map_message();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithMap::scanner()
        .fixed64_to_i32(SaveMap::with_value(Save))
        .fixed64_to_message(SaveMap::with_value(proto::MapValue::scanner().id(Save)))
        .string_to_i32(SaveMap::with_value(Save))
        .string_to_message(SaveMap::with_value(proto::MapValue::scanner().id(Save)));

    let output = scanner.scan(&mut bytes.as_slice()).read_all().unwrap();

    let expected = proto::ScanWithMapOutput::<_, _, _, _> {
        fixed64_to_i32: input
            .fixed64_to_i32
            .into_iter()
            .map(|(k, v)| (k, (v != 0).then_some(v)))
            .collect(),
        fixed64_to_message: input
            .fixed64_to_message
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    proto::ScanMapValueOutput {
                        id: (v.id != 0).then_some(v.id),
                    },
                )
            })
            .collect(),
        string_to_i32: input
            .string_to_i32
            .iter()
            .map(|(k, v)| (k.as_str().into(), (*v != 0).then_some(*v)))
            .collect(),
        string_to_message: input
            .string_to_message
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().into(),
                    proto::ScanMapValueOutput {
                        id: (v.id != 0).then_some(v.id),
                    },
                )
            })
            .collect(),
    };
    assert_eq!(output, expected)
}
