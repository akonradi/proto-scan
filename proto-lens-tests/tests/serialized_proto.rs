use std::collections::HashMap;
use std::sync::LazyLock;

use either::Either;
use prost::Message;
use proto_lens::wire::{
    FieldNumber, I32, LengthDelimited, ParseEvent, ParseEventReader, ScalarField, Varint,
};
use proto_lens_tests::proto;

const VARINT_VALUES: [u64; 3] = [19488045, 173485432435, 894];
const FIXED32_VALUES: [u32; 4] = [4, 5, 819491, 48];
const BOOL_VALUES: [bool; 5] = [true, true, false, false, true];
const ENUM_VALUES: [proto::with_repeats::EnumType; 3] = {
    use proto::with_repeats::EnumType;
    [EnumType::A, EnumType::B, EnumType::C]
};

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

static PACKED_FIELDS: LazyLock<HashMap<FieldNumber, Vec<ScalarField>>> = LazyLock::new(|| {
    HashMap::from([
        (
            FieldNumber::try_from(2).unwrap(),
            ENUM_VALUES
                .map(|e| ScalarField::Varint((e as u32).into()))
                .into(),
        ),
        (
            FieldNumber::try_from(3).unwrap(),
            BOOL_VALUES.map(|b| ScalarField::Varint(b.into())).into(),
        ),
        (
            FieldNumber::try_from(4).unwrap(),
            VARINT_VALUES.map(|b| ScalarField::Varint(b.into())).into(),
        ),
        (
            FieldNumber::try_from(6).unwrap(),
            FIXED32_VALUES.map(|b| ScalarField::I32(b.into())).into(),
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
        packed_enum: { ENUM_VALUES.map(Into::into).into() },
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
        let (field_number, event) = event.unwrap();
        match event {
            ParseEvent::Scalar(value) => events.entry(field_number).or_default().push(value),
            ParseEvent::StartGroup | ParseEvent::EndGroup | ParseEvent::LengthDelimited(_) => {}
        }
    }

    assert_eq!(events, *SCALAR_FIELDS,);
}

#[test]
fn extract_packed_fields_visitor() {
    let bytes = with_repeats().encode_to_vec();
    let mut events = HashMap::<_, Vec<_>>::new();

    let builder = proto_lens::visitor::Builder::new(&mut events).set_on_length_delimited(
        |events, field_number, value| {
            let iter = match field_number.into() {
                1 => return,
                2 | 3 | 4 => Either::Left(
                    value
                        .into_packed_varints()
                        .map(|r| ScalarField::Varint(r.unwrap())),
                ),
                6 => Either::Right(value.into_packed_i32s().map(|r| ScalarField::I32(r.unwrap()))),
                x => panic!("unexpected length-delimited field {x}"),
            };
            events.entry(field_number).or_default().extend(iter);
        },
    );

    let r = proto_lens::visitor::visit_message(
        proto_lens::wire::parse(&mut bytes.as_slice()),
        builder.build(),
    );
    assert_eq!(r, Ok(()));

    assert_eq!(events, *PACKED_FIELDS);
}

#[test]
fn extract_packed_fields_parse() {
    let bytes = with_repeats().encode_to_vec();
    let mut events = HashMap::<_, Vec<_>>::new();

    let mut read = bytes.as_slice();
    let mut reader = proto_lens::wire::parse(&mut read);
    while let Some(event) = reader.next() {
        let (field_number, event) = event.unwrap();
        let values = match event {
            ParseEvent::LengthDelimited(value) => match field_number.into() {
                2 | 3 | 4 => {
                    let it = value
                        .into_packed::<Varint>()
                        .map(|r| ScalarField::Varint(r.unwrap()));
                    Either::Left(it)
                }
                6 => {
                    let it = value
                        .into_packed::<I32>()
                        .map(|r| ScalarField::I32(r.unwrap()));
                    Either::Right(it)
                }
                1 => continue,
                field_number => panic!("unknown field {field_number}"),
            },
            ParseEvent::Scalar(_) | ParseEvent::StartGroup | ParseEvent::EndGroup => {
                continue;
            }
        };
        events.entry(field_number).or_default().extend(values);
    }

    assert_eq!(events, *PACKED_FIELDS);
}

#[test]
fn extract_string() {
    let bytes = with_repeats().encode_to_vec();
    let mut read = bytes.as_slice();
    let mut reader = proto_lens::wire::parse(&mut read);
    while let Some(event) = reader.next() {
        let (field_number, event) = event.unwrap();
        match event {
            ParseEvent::LengthDelimited(l) => {
                if field_number == 1 {
                    let mut reader = l.into_events();
                    while let Some(event) = reader.next() {
                        let (_field_number, event) = event.unwrap();
                        match event {
                            ParseEvent::LengthDelimited(l) => {
                                if field_number == 1 {
                                    let bytes = l.into_bytes().unwrap();
                                    let bytes = bytes.as_ref();
                                    assert_eq!(bytes, b"ABC");
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            ParseEvent::Scalar(_) => {}
            ParseEvent::StartGroup => {}
            ParseEvent::EndGroup => {}
        }
    }
}
