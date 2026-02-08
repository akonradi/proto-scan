use std::collections::HashMap;
use std::sync::LazyLock;

use prost::Message;
use proto_lens::wire::{FieldNumber, LengthDelimited, ParseEvent, ParseEventReader, ScalarField};
use proto_lens_tests::proto;

const VARINT_VALUES: [u64; 3] = [19488045, 173485432435, 894];
const FIXED32_VALUES: [u32; 4] = [4, 5, 819491, 48];
const BOOL_VALUES: [bool; 5] = [true, true, false, false, true];

static SCALAR_FIELDS: LazyLock<HashMap<FieldNumber, Vec<ScalarField>>> = LazyLock::new(|| {
    HashMap::from([
        (
            FieldNumber::try_from(5).unwrap(),
            VARINT_VALUES.map(ScalarField::Varint).into(),
        ),
        (
            FieldNumber::try_from(7).unwrap(),
            FIXED32_VALUES.map(ScalarField::I32).into(),
        ),
    ])
});

fn with_repeats() -> proto::WithRepeats {
    proto::WithRepeats {
        messages: vec![proto::MultiFieldMessage {
            id: 1,
            name: "ABC".to_string(),
        }],
        packed_bool: BOOL_VALUES.into(),
        packed_enum: {
            use proto::with_repeats::EnumType;
            [EnumType::A, EnumType::B, EnumType::C]
                .map(Into::into)
                .to_vec()
        },
        packed_varint: VARINT_VALUES.into(),
        unpacked_varint: VARINT_VALUES.into(),
        packed_fixed32: FIXED32_VALUES.into(),
        unpacked_fixed32: FIXED32_VALUES.into(),
    }
}

#[test]
fn extract_scalars_visitor() {
    let bytes = with_repeats().encode_to_vec();
    let mut events = HashMap::<_, Vec<_>>::new();

    let builder = proto_lens::visitor::Builder::new(&mut events).set_on_scalar(
        |events, field_number, value| {
            events.entry(field_number).or_default().push(value);
        },
    );

    let r = proto_lens::visitor::visit_message(
        proto_lens::wire::parse(&mut bytes.as_slice()),
        builder.build(),
    );
    assert_eq!(r, Ok(()));

    assert_eq!(events, *SCALAR_FIELDS);
}

#[test]
fn extract_scalars_parse() {
    let mut events = HashMap::<_, Vec<_>>::new();

    let bytes = with_repeats().encode_to_vec();
    let mut read = bytes.as_slice();
    let mut reader = proto_lens::wire::parse(&mut read);
    while let Some(event) = reader.next() {
        let event = event.unwrap();
        match event {
            ParseEvent::Scalar(field_number, value) => {
                events.entry(field_number).or_default().push(value)
            }
            ParseEvent::StartGroup(_field_number)
            | ParseEvent::EndGroup(_field_number)
            | ParseEvent::LengthDelimited(_field_number, _) => {}
        }
    }

    assert_eq!(events, *SCALAR_FIELDS,);
}

#[test]
fn extract_string() {
    let bytes = with_repeats().encode_to_vec();
    let mut read = bytes.as_slice();
    let mut reader = proto_lens::wire::parse(&mut read);
    while let Some(event) = reader.next() {
        let event = event.unwrap();
        match event {
            ParseEvent::LengthDelimited(field_number, l) => {
                if field_number == 1 {
                    let mut reader = l.as_events();
                    while let Some(event) = reader.next() {
                        match event.unwrap() {
                            ParseEvent::LengthDelimited(f, l) => {
                                if f == 1 {
                                    let bytes = l.as_bytes().unwrap();
                                    let bytes = bytes.as_ref();
                                    assert_eq!(bytes, b"ABC");
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            ParseEvent::Scalar(_, _) => {}
            ParseEvent::StartGroup(_) => {}
            ParseEvent::EndGroup(_) => {}
        }
    }
}
