use proto_scan_tests::{prost_proto, proto};

use prost::Message as _;
use proto_scan::scan::{Scan as _, ScanMessage as _, Scanner as _};

fn example_msg() -> prost_proto::ScanExample {
    prost_proto::ScanExample {
        repeated_msg: vec![prost_proto::MultiFieldMessage {
            id: 1,
            name: "ABC".to_string(),
        }],
        single_msg: Some(prost_proto::MultiFieldMessage {
            name: "a".to_owned(),
            id: 2,
        }),
        repeated_bool: vec![true, true, false, false],
        single_bool: Some(true),
        oneof_group: Some(prost_proto::scan_example::OneofGroup::OneofFixed32(
            11111111,
        )),
        single_fixed64: Some(123456789),
    }
}

mod save_single_bool {
    use super::*;

    #[test]
    fn empty() {
        let mut save_to = None;

        let scanner = proto::testing::ScanExample::scanner().save_single_bool(&mut save_to);
        {
            for event in scanner.scan([].as_slice()) {
                match Result::unwrap(event) {
                    None => {}
                }
            }
        }

        assert_eq!(save_to, None);
    }

    #[test]
    fn with_field() {
        let mut save_to = None;

        let scanner = proto::testing::ScanExample::scanner().save_single_bool(&mut save_to);
        {
            let message = example_msg().encode_to_vec();
            for event in scanner.scan(message.as_slice()) {
                match Result::unwrap(event) {
                    None => {}
                }
            }
        }

        assert_eq!(save_to, example_msg().single_bool);
    }
}

mod emit_single_bool {
    use super::*;

    #[test]
    fn empty() {
        let mut save_to = None::<bool>;

        let scanner = proto::testing::ScanExample::scanner().emit_single_bool();
        {
            for event in scanner.scan([].as_slice()) {
                match Result::unwrap(event) {
                    Some(proto::testing::ScanScanExampleEvent::Event3(b)) => {
                        save_to = Some(b);
                    }
                    None => {}
                }
            }
        }

        assert_eq!(save_to, None);
    }

    #[test]
    fn with_field() {
        let mut save_to = None;

        let scanner = proto::testing::ScanExample::scanner().emit_single_bool();
        {
            let message = example_msg().encode_to_vec();
            for event in scanner.scan(message.as_slice()) {
                match Result::unwrap(event) {
                    Some(proto::testing::ScanScanExampleEvent::Event3(b)) => save_to = Some(b),
                    None => {}
                }
            }
        }

        assert_eq!(save_to, example_msg().single_bool);
    }
}

mod read_all {
    use super::*;

    #[test]
    fn empty() {
        let scanner = proto::testing::ScanExample::scanner()
            .emit_single_bool()
            .emit_single_fixed64();
        let scan = scanner.scan([].as_slice());
        let read_all = scan.read_all();
        let proto::testing::ScanScanExampleOutput {
            single_bool,
            repeated_msg: (),
            single_msg: (),
            repeated_bool: (),
            oneof_group: (),
            single_fixed64,
        } = read_all.unwrap();

        assert_eq!(single_bool, None);
        assert_eq!(single_fixed64, None);
    }

    #[test]
    fn with_field() {
        let message = example_msg();
        let mut save_to = (None, None);

        let scanner = proto::testing::ScanExample::scanner()
            .save_single_bool(&mut save_to.0)
            .save_single_fixed64(&mut save_to.1);
        let proto::testing::ScanScanExampleOutput {
            repeated_msg: (),
            single_msg: (),
            repeated_bool: (),
            single_bool: (),
            oneof_group: (),
            single_fixed64: (),
        } = scanner
            .scan(message.encode_to_vec().as_slice())
            .read_all()
            .unwrap();

        assert_eq!(save_to, (message.single_bool, message.single_fixed64));
    }
}
