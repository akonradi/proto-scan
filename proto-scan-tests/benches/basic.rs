use std::collections::HashMap;
use std::hash::RandomState;
use std::hint::black_box;
use std::sync::LazyLock;

use gungraun::{LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main};

use prost::Message as _;
use proto_scan::scan::field::{Save, ScanOptionalMessage as _};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder};
use proto_scan::wire::{LengthDelimited as _, ParseEvent, ParseEventReader as _};
use proto_scan_tests::{prost_proto, proto};

fn byte_types_inputs() -> &'static [u8] {
    static BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
        prost_proto::BytesFieldTypes {
            bytes_field: b"bytes".repeat(10),
            string_field: "string".repeat(10),
            ..Default::default()
        }
        .encode_to_vec()
    });

    &BYTES
}

#[library_benchmark]
#[bench::byte_types(byte_types_inputs())]
fn byte_types(encoded: &[u8]) -> usize {
    let scanner = proto::BytesFieldTypes::scanner()
        .bytes_field(Save)
        .string_field(Save);

    let output = scanner.scan(black_box(encoded)).read_all().unwrap();
    output.bytes_field.len() + output.string_field.len()
}

fn embedded_message_input() -> &'static [u8] {
    static BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
        prost_proto::ScanExample {
            repeated_bool: vec![true, false, false],
            single_msg: Some(prost_proto::MultiFieldMessage {
                flag: true,
                id: 32918495,
                name: "name for a message".into(),
            }),
            ..Default::default()
        }
        .encode_to_vec()
    });

    &BYTES
}

#[library_benchmark]
#[bench::embedded_message(embedded_message_input())]
fn embedded_message(encoded: &[u8]) -> usize {
    let scanner = proto::ScanExample::scanner().single_msg(
        proto::MultiFieldMessage::scanner()
            .flag(Save)
            .name(Save)
            .empty_if_not_present(),
    );

    let output = scanner.scan(black_box(encoded)).read_all().unwrap();
    output.single_msg.name.len()
}

static LENGTH_DELIMITED_STACK_INPUT: &[u8] = &[
    0x12, 0x7, 0x1a, 0x5, 0xd, 0x1, 0x2, 0x3, 0x4, 0xd, 0x1, 0x2, 0x3, 0x4,
];

fn large_message_input() -> &'static [u8] {
    static BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
        let uninteresting = prost_proto::Any {
            type_url: "some long URL to be checked but not matched".into(),
            value: b"contents that are present and take up space but aren't actually important"
                .into(),
        };
        let target_contents = prost_proto::ScanExample {
            single_msg: Some(prost_proto::MultiFieldMessage {
                name: "the name".into(),
                id: 1234,
                flag: true,
            }),
            ..Default::default()
        }
        .encode_to_vec();
        let target = prost_proto::Any {
            type_url: "the matching type URL".into(),
            value: target_contents,
        };
        let mut stable_ordering_map = {
            let state: RandomState = unsafe { std::mem::zeroed() };
            HashMap::with_hasher(state)
        };
        stable_ordering_map.extend(
            (0..200)
                .map(|i| (format!("ignored key {i}"), uninteresting.clone()))
                .chain([("the needle".into(), target)]),
        );
        prost_proto::AnyMap {
            values: stable_ordering_map,
        }
        .encode_to_vec()
    });

    &BYTES
}

#[library_benchmark]
#[bench::large_message(large_message_input())]
fn large_message(encoded: &[u8]) -> usize {
    let output = proto::AnyMap::scanner()
        .values(Save::map_value(
            "the needle",
            proto::Any::scanner().type_url(Save).value(Save),
        ))
        .scan(encoded)
        .read_all()
        .unwrap();
    let proto::ScanAnyOutput { type_url, value } = output.values.unwrap();
    assert_eq!(type_url, "the matching type URL");
    let proto::ScanScanExampleOutput {
        single_msg,
        repeated_msg: (),
        repeated_bool: (),
        single_bool: (),
        single_fixed64: (),
        oneof_group: (),
    } = proto::ScanExample::scanner()
        .single_msg(
            proto::MultiFieldMessage::scanner()
                .name(Save)
                .id(Save)
                .flag(Save),
        )
        .scan(value)
        .read_all()
        .unwrap();
    single_msg.unwrap().name.len()
}

#[library_benchmark]
#[bench::length_delimited_stack(LENGTH_DELIMITED_STACK_INPUT)]
fn length_delimited_stack(input: &[u8]) {
    let mut reader = proto_scan::wire::parse(input);

    let mut read_mid = match reader.next().unwrap().unwrap() {
        (f, ParseEvent::LengthDelimited(l)) => {
            assert_eq!(f, 2);
            l
        }
        _ => panic!("wrong event"),
    }
    .into_events();

    let read_inner = match read_mid.next().unwrap().unwrap() {
        (f, ParseEvent::LengthDelimited(l)) => {
            assert_eq!(f, 3);
            l
        }
        _ => panic!("wrong event"),
    };

    drop(read_inner);
    drop(read_mid);
}

library_benchmark_group!(
    name = bench_basic,
    config = LibraryBenchmarkConfig::default().valgrind_args(["--dump-instr=yes"]),
    benchmarks = [
        byte_types,
        embedded_message,
        length_delimited_stack,
        large_message
    ]
);

main!(library_benchmark_groups = bench_basic);
