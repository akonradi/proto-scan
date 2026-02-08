use std::collections::HashMap;

use prost::Message;
use proto_lens::visitor::ScalarField;
use proto_lens_tests::proto;

const VARINT_VALUES: [u64; 3] = [19488045, 173485432435, 894];
const FIXED32_VALUES: [u32; 4] = [4, 5, 819491, 48];
const BOOL_VALUES: [bool; 5] = [true, true, false, false, true];

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
fn extract_scalars() {
    let bytes = with_repeats().encode_to_vec();
    let mut events = HashMap::<_, Vec<_>>::new();

    let builder = proto_lens::visitor::Builder::new(&mut events).set_on_scalar(
        |events, field_number, value| {
            events.entry(field_number).or_default().push(value);
        },
    );

    let r = proto_lens::visitor::visit_message(&mut bytes.as_slice(), builder.build());
    assert_eq!(r, Ok(()));

    assert_eq!(
        events,
        HashMap::from([
            (5, VARINT_VALUES.map(ScalarField::Varint).into()),
            (7, FIXED32_VALUES.map(ScalarField::I32).into()),
        ])
    );
}
