use prost::Message as _;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

impl InputKind {
    fn into_recursive(self) -> crate::prost_proto::Recursive {
        use crate::prost_proto::{Recursive, recursive::Kind};

        match self {
            Self::Empty => Default::default(),
            Self::Full => Recursive {
                id: 1,
                child: Some(Box::new(Recursive {
                    id: 2,
                    ..Default::default()
                })),
                kind: Some(Kind::BoxedOneofChild(Box::new(Recursive {
                    id: 3,
                    ..Default::default()
                }))),
            },
        }
    }
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_boxed_message_field(input: InputKind) {
    let input = input.into_recursive();
    let bytes = input.encode_to_vec();

    let scanner = proto::Recursive::scanner()
        .id(Save)
        .child(proto::Recursive::scanner().id(Save));
    let scan = scanner.scan(bytes.as_slice());

    let proto::ScanRecursiveOutput {
        id,
        child,
        kind: (),
    } = scan.read_all().unwrap();

    assert_eq!(id, input.id);
    assert_eq!(
        child,
        input.child.map(|c| proto::ScanRecursiveOutput {
            id: c.id,
            child: (),
            kind: (),
        })
    );
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_boxed_oneof_message_field(input: InputKind) {
    let input = input.into_recursive();
    let bytes = input.encode_to_vec();

    let scanner = proto::Recursive::scanner().kind(
        proto::recursive::Kind::scanner().boxed_oneof_child(proto::Recursive::scanner().id(Save)),
    );
    let scan = scanner.scan(bytes.as_slice());

    let proto::ScanRecursiveOutput {
        id: (),
        child: (),
        kind,
    } = scan.read_all().unwrap();

    assert_eq!(
        kind,
        input.kind.map(|k| match k {
            crate::prost_proto::recursive::Kind::Leaf(_) => unreachable!(),
            crate::prost_proto::recursive::Kind::BoxedOneofChild(c) =>
                proto::recursive::ScanKindOutput::BoxedOneofChild(proto::ScanRecursiveOutput {
                    id: c.id,
                    child: (),
                    kind: (),
                }),
        })
    );
}
