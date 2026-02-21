use proc_macro2::{Span, TokenStream};
use prost_build::Module;
use prost_types::DescriptorProto;
use prost_types::FileDescriptorProto;
use proto_scan_gen::ScannableMessage;
use proto_scan_gen::field::{FieldType, MessageField, SingleFieldType};
use quote::quote;
use std::io::Result;
use syn::Ident;

pub(crate) fn generate_module(module: &Module, fd: FileDescriptorProto) -> Result<String> {
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

    let parts = module
        .parts()
        .map(|m| format!("pub mod {m} {{"))
        .chain(messages)
        .chain(module.parts().map(|_| "}".to_owned()))
        .collect::<Vec<_>>();

    Ok(parts.join("\n"))
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

    let fields = field
        .iter()
        .enumerate()
        .map(|(i, value)| {
            use prost_types::field_descriptor_proto::{Label, Type};

            let label = value.label();
            let field_type = match (value.r#type(), label) {
                (Type::Bool, Label::Optional | Label::Required) => {
                    FieldType::Single(SingleFieldType::Bool)
                }
                (Type::Fixed64, Label::Optional | Label::Required) => {
                    FieldType::Single(SingleFieldType::FixedU64)
                }
                (
                    Type::Double
                    | Type::Float
                    | Type::Int64
                    | Type::Uint64
                    | Type::Int32
                    | Type::Fixed64
                    | Type::Fixed32
                    | Type::Bool
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

            let field_number = value.number().try_into().map_err(|_| {
                std::io::Error::other(format!("invalid field number {}", value.number()))
            })?;

            Ok(MessageField {
                field_name: ident(value.name()),
                field_number,
                field_type,
                generic: ident(format!("T{i}")),
            })
        })
        .collect::<Result<_>>()?;

    Ok(ScannableMessage {
        name,
        fields: fields,
    })
}
