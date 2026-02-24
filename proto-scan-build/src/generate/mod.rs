use itertools::Itertools as _;
use proc_macro2::{Span, TokenStream};
use prost_types::FileDescriptorProto;
use prost_types::{DescriptorProto, OneofDescriptorProto};
use proto_scan_gen::ScannableMessage;
use proto_scan_gen::field::{
    FieldType, FixedFieldType, MessageField, ParsedFieldType, VarintFieldType,
};
use quote::quote;
use std::collections::HashMap;
use std::io::Result;
use syn::Ident;

pub(crate) fn generate_module(fd: FileDescriptorProto) -> Result<String> {
    let FileDescriptorProto {
        name: _,
        package: _,
        dependency: _,
        public_dependency: _,
        weak_dependency: _,
        message_type,
        enum_type: _,
        service: _,
        extension: _,
        options: _,
        source_code_info: _,
        syntax: _,
    } = fd;

    let messages = message_type
        .iter()
        .map(generate_message)
        .collect::<Result<Vec<_>>>()?;

    Ok(messages.join("\n"))
}

fn generate_message(message: &DescriptorProto) -> Result<String> {
    let message = message_from_descriptor(message)?;
    let scanner = message.scanner();

    let name = &message.name;
    let type_defn = quote! {
        pub struct #name;
    };

    Ok([type_defn, scanner.generated_code()]
        .into_iter()
        .collect::<TokenStream>()
        .to_string())
}

fn ident(s: impl AsRef<str>) -> Ident {
    Ident::new(s.as_ref(), Span::call_site())
}

fn message_from_descriptor(message: &DescriptorProto) -> Result<ScannableMessage> {
    let DescriptorProto {
        name,
        field,
        oneof_decl,
        extension: _,
        nested_type: _,
        enum_type: _,
        extension_range: _,
        options: _,
        reserved_range: _,
        reserved_name: _,
    } = message;

    let name = ident(
        name.as_ref()
            .ok_or_else(|| std::io::Error::other("missing name"))?,
    );

    let oneofs_to_fields = {
        let mut map = HashMap::<_, Vec<_>>::new();
        for index in field.iter().filter_map(|f| f.oneof_index) {
            map.entry(index).or_default().push(index);
        }
        map
    };

    let field_types = field
        .iter()
        .map(|value| {
            if value
                .oneof_index
                .is_some_and(|index| oneofs_to_fields.get(&index).is_some_and(|v| v.len() > 1))
            {
                return Ok(None);
            }

            let field_type = extract_field_type(value)?;

            Ok(Some((ident(value.name()), field_type)))
        })
        .flatten_ok()
        .chain(oneof_decl.iter().zip(0i32..).filter_map(|(oneof, i)| {
            let OneofDescriptorProto { name, options: _ } = oneof;
            if oneofs_to_fields.get(&i).is_some_and(|v| v.len() == 1) {
                return None;
            }
            Some(Ok((
                ident(name.as_deref().unwrap_or_default()),
                FieldType::Unsupported,
            )))
        }))
        .collect::<Result<Vec<_>>>()?;

    let fields = field_types
        .into_iter()
        .enumerate()
        .map(|(i, (name, field_type))| MessageField {
            field_name: name,
            generic: ident(format!("T{i}")),
            field_type,
        })
        .collect();
    Ok(ScannableMessage { name, fields })
}

fn extract_field_type(value: &prost_types::FieldDescriptorProto) -> Result<FieldType> {
    use prost_types::field_descriptor_proto::{Label, Type};
    let label = value.label();

    let parsed_field_type: ParsedFieldType = match value.r#type() {
        Type::Int64 => VarintFieldType::I64.into(),
        Type::Uint64 => VarintFieldType::U64.into(),
        Type::Int32 => VarintFieldType::I32.into(),
        Type::Fixed64 => FixedFieldType::U64.into(),
        Type::Fixed32 => FixedFieldType::U32.into(),
        Type::Bool => VarintFieldType::Bool.into(),
        Type::Uint32 => VarintFieldType::U32.into(),
        Type::Sfixed32 => FixedFieldType::I32.into(),
        Type::Sfixed64 => FixedFieldType::I64.into(),
        Type::Sint32 => VarintFieldType::I32Z.into(),
        Type::Sint64 => VarintFieldType::I64Z.into(),
        Type::Float => FixedFieldType::F32.into(),
        Type::Double => FixedFieldType::F64.into(),
        Type::Message => ParsedFieldType::Message,
        Type::Bytes => ParsedFieldType::Bytes { utf8: false },
        Type::String => ParsedFieldType::Bytes { utf8: true },
        Type::Enum | Type::Group => {
            return Ok(FieldType::Unsupported);
        }
    };
    let number = value
        .number()
        .try_into()
        .map_err(|_| std::io::Error::other(format!("invalid field number {}", value.number())))?;

    Ok(match (parsed_field_type, label) {
        (ParsedFieldType::Single(single), Label::Optional | Label::Required) => {
            FieldType::Single { ty: single, number }
        }
        (ParsedFieldType::Single(single), Label::Repeated) => {
            FieldType::Repeated { ty: single, number }
        }
        (ParsedFieldType::Message, Label::Optional | Label::Required) => {
            FieldType::Message { number }
        }
        (ParsedFieldType::Bytes { utf8 }, Label::Optional | Label::Required) => {
            FieldType::Bytes { utf8, number }
        }
        (ParsedFieldType::Message, Label::Repeated)
        | (ParsedFieldType::Bytes { utf8: _ }, Label::Repeated) => FieldType::Unsupported,
    })
}
