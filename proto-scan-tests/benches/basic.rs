use std::hint::black_box;
use std::sync::LazyLock;

use gungraun::{library_benchmark, library_benchmark_group, main};

use prost::Message as _;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder};
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

library_benchmark_group!(name = bench_basic, benchmarks = byte_types);

main!(library_benchmark_groups = bench_basic);
