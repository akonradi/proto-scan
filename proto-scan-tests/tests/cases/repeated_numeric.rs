use prost::Message;
use proto_scan::scan::field::Write;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn save_repeated_bool(input: InputKind) {
    let input = input.into_example_msg();
    let mut save_to = vec![true, true];

    let scanner = proto::ScanExample::scanner().repeated_bool(Write(&mut save_to));
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                None => {}
            }
        }
    }
    assert_eq!(
        save_to,
        [true, true]
            .into_iter()
            .chain(input.repeated_bool)
            .collect::<Vec<_>>()
    );
}
