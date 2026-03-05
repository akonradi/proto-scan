use std::sync::LazyLock;

use prost::Message;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage, ScannerBuilder};

use super::*;

static FIRST_MESSAGE: LazyLock<crate::prost_proto::ScanExample> = LazyLock::new(|| {
    use crate::prost_proto::*;
    ScanExample {
        repeated_msg: vec![MultiFieldMessage {
            id: 1,
            ..Default::default()
        }],
        single_msg: Some(MultiFieldMessage {
            name: "first".to_owned(),
            id: 2,
            ..Default::default()
        }),
        ..Default::default()
    }
});

static SECOND_MESSAGE: LazyLock<crate::prost_proto::ScanExample> = LazyLock::new(|| {
    use crate::prost_proto::*;
    ScanExample {
        single_msg: Some(MultiFieldMessage {
            name: "second".to_owned(),
            flag: true,
            ..Default::default()
        }),
        ..Default::default()
    }
});

static MERGED_MESSAGE: LazyLock<crate::prost_proto::ScanExample> = LazyLock::new(|| {
    use crate::prost_proto::*;
    ScanExample {
        repeated_msg: vec![MultiFieldMessage {
            id: 1,
            ..Default::default()
        }],
        single_msg: Some(MultiFieldMessage {
            name: "second".to_owned(),
            id: 2,
            flag: true,
        }),
        ..Default::default()
    }
});

#[test]
fn prost_merged_parsing() {
    let parsed = crate::prost_proto::ScanExample::decode(&*{
        let mut e = FIRST_MESSAGE.encode_to_vec();
        e.extend(SECOND_MESSAGE.encode_to_vec());
        e
    })
    .unwrap();

    assert_eq!(parsed, *MERGED_MESSAGE);
}

#[test]
fn scan_merged_parsing() {
    let joined = {
        let mut e = FIRST_MESSAGE.encode_to_vec();
        e.extend(SECOND_MESSAGE.encode_to_vec());
        e
    };
    let contents = [joined, MERGED_MESSAGE.encode_to_vec()];
    let [separate, merged] = contents.each_ref().map(|bytes| {
        let scanner = proto::ScanExample::scanner().single_msg(
            proto::MultiFieldMessage::scanner()
                .flag(Save)
                .id(Save)
                .name(Save),
        );
        scanner.scan(bytes.as_slice()).read_all().unwrap()
    });

    assert_eq!(separate, merged);
}
