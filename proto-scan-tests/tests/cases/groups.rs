use prost::Message as _;
use proto_scan::scan::field::{Save, SaveCloned, ScanRepeated as _};
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};
use test_case::test_case;

use super::*;
use InputKind::*;

impl InputKind {
    fn into_groups_message(self) -> crate::prost_proto::WithGroup {
        use crate::prost_proto::with_group;
        match self {
            Empty => Default::default(),
            Full => crate::prost_proto::WithGroup {
                groupfield: vec![
                    with_group::GroupField {
                        field_1: Some("a".into()),
                        field_2: Some("b".into()),
                    },
                    with_group::GroupField {
                        field_1: Some("c".into()),
                        field_2: Some("d".into()),
                    },
                ],
            },
        }
    }
}

#[test_case(Empty)]
#[test_case(Full)]
fn scan_message(input: InputKind) {
    let input = input.into_groups_message();
    let bytes = input.encode_to_vec();

    let scanner = proto::WithGroup::scanner().groupfield(
        proto::with_group::GroupField::scanner()
            .field_1(Save)
            .field_2(Save)
            .repeat_by(SaveCloned),
    );

    let output = scanner
        .scan(&mut bytes.as_slice())
        .with_group_stack(Vec::new())
        .read_all()
        .unwrap();

    assert_eq!(
        output,
        proto::ScanWithGroupOutput {
            groupfield: input
                .groupfield
                .iter()
                .map(|g| proto::with_group::ScanGroupFieldOutput {
                    field_1: g.field_1.as_deref().unwrap_or_default(),
                    field_2: g.field_2.as_deref().unwrap_or_default(),
                })
                .collect()
        }
    )
}
