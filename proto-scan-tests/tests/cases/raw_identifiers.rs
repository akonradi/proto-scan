use prost::Message as _;
use proto_scan::scan::field::Save;
use proto_scan::scan::{ScanMessage as _, ScannerBuilder as _};

use super::*;

#[test]
fn save_field() {
    let input = crate::prost_proto::WithKeywords {
        r#as: "as".into(),
        r#type: 1,
        r#impl: b"impl".into(),
        r#enum: Some(proto_scan_tests::prost_proto::with_keywords::Enum::Loop(
            false,
        )),
    };
    let bytes = input.encode_to_vec();

    let scanner = proto::WithKeywords::scanner()
        .r#as(Save)
        .r#type(Save)
        .r#impl(Save)
        .r#enum(proto::with_keywords::Enum::scanner().r#loop(Save));
    let scan = scanner.scan(bytes.as_slice());

    let read_all = scan.read_all();
    let proto::ScanWithKeywordsOutput {
        r#as,
        r#type,
        r#impl,
        r#enum,
    } = read_all.unwrap();

    assert_eq!(r#as, input.r#as);
    assert_eq!(r#type, input.r#type);
    assert_eq!(r#impl, input.r#impl);
    assert_eq!(
        r#enum,
        input.r#enum.map(|e| match e {
            crate::prost_proto::with_keywords::Enum::Loop(b) =>
                crate::proto::with_keywords::ScanEnumOutput::Loop(b),
        })
    )
}
