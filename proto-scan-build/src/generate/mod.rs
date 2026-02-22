use itertools::Itertools as _;
use proc_macro2::{Span, TokenStream};
use prost_types::FileDescriptorProto;
use prost_types::{DescriptorProto, OneofDescriptorProto};
use proto_scan_gen::ScannableMessage;
use proto_scan_gen::field::{FieldType, MessageField, SingleFieldType};
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
        extension,
        nested_type,
        enum_type,
        extension_range,
        oneof_decl,
        options,
        reserved_range,
        reserved_name,
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
            use prost_types::field_descriptor_proto::{Label, Type};

            if let Some(index) = value.oneof_index
                && oneofs_to_fields.get(&index).is_some_and(|v| v.len() > 1)
            {
                return Ok(None);
            }

            let label = value.label();
            let field_number = || {
                value.number().try_into().map_err(|_| {
                    std::io::Error::other(format!("invalid field number {}", value.number()))
                })
            };
            let field_type = match (value.r#type(), label) {
                (Type::Bool, Label::Optional | Label::Required) => FieldType::Single {
                    ty: SingleFieldType::Bool,
                    number: field_number()?,
                },
                (Type::Fixed64, Label::Optional | Label::Required) => FieldType::Single {
                    ty: SingleFieldType::FixedU64,
                    number: field_number()?,
                },
                (Type::Bool, Label::Repeated) => FieldType::Repeated {
                    ty: SingleFieldType::Bool,
                    number: field_number()?,
                },
                (Type::Fixed64, Label::Repeated) => FieldType::Repeated {
                    ty: SingleFieldType::FixedU64,
                    number: field_number()?,
                },
                (Type::Message, Label::Optional | Label::Required) => FieldType::Message {
                    number: field_number()?,
                },
                (
                    Type::Double
                    | Type::Float
                    | Type::Int64
                    | Type::Uint64
                    | Type::Int32
                    | Type::Fixed32
                    | Type::String
                    | Type::Group
                    | Type::Message
                    | Type::Bytes
                    | Type::Uint32
                    | Type::Enum
                    | Type::Sfixed32
                    | Type::Sfixed64
                    | Type::Sint32
                    | Type::Sint64,
                    Label::Optional | Label::Repeated | Label::Required,
                ) => FieldType::Unsupported,
            };

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
