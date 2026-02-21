use prost::Message;
use proto_scan::scan::{ScanMessage as _, Scanner as _};
use test_case::test_case;

use super::*;
use InputKind::*;

#[test_case(Empty)]
#[test_case(Full)]
fn save_single_bool(input: InputKind) {
    let input = input.into_example_msg();
    let mut save_to = None;

    let scanner = proto::ScanExample::scanner().save_single_bool(&mut save_to);
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                None => {}
            }
        }
    }
    assert_eq!(save_to, input.single_bool);
}

#[test_case(Empty)]
#[test_case(Full)]
fn emit_single_bool(input: InputKind) {
    let input = input.into_example_msg();
    let mut save_to = None::<bool>;

    let scanner = proto::ScanExample::scanner().emit_single_bool();
    {
        let bytes = input.encode_to_vec();
        for event in scanner.scan(bytes.as_slice()) {
            match Result::unwrap(event) {
                Some(proto::ScanScanExampleEvent::Event3(b)) => {
                    save_to = Some(b);
                }
                None => {}
            }
        }
    }

    assert_eq!(save_to, input.single_bool);
}
