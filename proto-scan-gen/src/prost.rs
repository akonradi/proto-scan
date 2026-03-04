use std::borrow::Cow;

use convert_case::ccase;
use proc_macro2::{Span, TokenStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::{Attribute, DataEnum, DataStruct, DeriveInput, Expr, Ident, Meta, Result, Token};

use crate::field::{
    BytesField, Field, FixedFieldType, MessageField, MessageFieldType, OneOfField, ParsedFieldType,
    RepeatedField, RepeatedFieldType, SingleField, VarintFieldType,
};
use crate::message::ScannableMessage;
use crate::oneof::ScannableOneof;

pub fn derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        vis: _,
    } = input;

    if !generics.params.is_empty() {
        return Err(syn::Error::new(
            generics.span(),
            "generics are not supported",
        ));
    }

    match data {
        syn::Data::Struct(data_struct) => message_impl(ident, data_struct),
        syn::Data::Enum(data_enum)
            if attrs.iter().find(|a| a.path().is_ident("repr")).is_none() =>
        {
            enum_impl(ident, data_enum)
        }
        syn::Data::Enum(_) => Ok(TokenStream::new()),
        syn::Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "union types are not supported",
        )),
    }
}

fn enum_impl(name: Ident, data_enum: DataEnum) -> Result<TokenStream> {
    let DataEnum {
        brace_token: _,
        enum_token: _,
        variants,
    } = data_enum;
    let fields = variants
        .into_iter()
        .enumerate()
        .map(|(i, field)| {
            let span = field.span();
            let syn::Variant {
                attrs,
                ident: variant_name,
                fields,
                discriminant: _,
            } = field;
            let field = match fields {
                syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    fields.unnamed.into_iter().next().unwrap()
                }
                syn::Fields::Unnamed(_) | syn::Fields::Named(_) | syn::Fields::Unit => {
                    return Err(syn::Error::new(
                        variant_name.span(),
                        "expected a single unnamed field",
                    ));
                }
            };
            let syn::Field {
                attrs: _,
                vis: _,
                mutability: _,
                ident: _,
                colon_token: _,
                ty,
            } = field;

            let ProstAttrs { field_type } = (attrs, ty).try_into()?;
            let field_type = match field_type {
                MessageFieldType::Single(single_field) => OneOfField::Single(single_field),
                MessageFieldType::Bytes(bytes_field) => OneOfField::Bytes(bytes_field),
                MessageFieldType::Message(message_field) => OneOfField::Message(message_field),
                MessageFieldType::Repeated { .. }
                | MessageFieldType::OneOf { .. }
                | MessageFieldType::Unsupported => {
                    return Err(syn::Error::new(
                        span,
                        format!("oneof has {field_type:?} field"),
                    ));
                }
            };

            let generic = Ident::new(&format!("T{i}"), span);
            let field_name = Ident::new(&ccase!(snake, variant_name.to_string()), span);

            Ok(Field {
                field_name,
                variant_name,
                field_type,
                generic,
            })
        })
        .collect::<Result<_>>()?;
    let oneof = ScannableOneof { name, fields };
    let scanner = oneof.scanner();
    Ok([
        scanner.scanner_type_definition(),
        scanner.output_type_definition(),
        scanner.event_type_definition(),
        scanner.field_number_type_definition(),
        scanner.field_number_type_impls(),
        oneof.impl_scan_message(),
        oneof.impl_scannable_oneof(),
        scanner.impl_scanner_builder(),
        scanner.impl_scan_callbacks(),
    ]
    .into_iter()
    .chain(scanner.fields().map(|f| f.impl_()))
    .collect())
}

fn message_impl(name: Ident, data_struct: DataStruct) -> Result<TokenStream> {
    let fields = data_struct
        .fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| {
            let span = field.span();
            let syn::Field {
                attrs,
                ident: field_name,
                vis: _,
                mutability: _,
                colon_token: _,
                ty,
            } = field;

            let field_name =
                field_name.ok_or_else(|| syn::Error::new(span, "message fields must be named"))?;
            let variant_name = Ident::new(&ccase!(pascal, field_name.to_string()), span);

            let ProstAttrs { field_type } = (attrs, ty).try_into()?;

            let generic = Ident::new(&format!("T{i}"), Span::call_site());

            Ok(Field {
                field_name,
                variant_name,
                field_type,
                generic,
            })
        })
        .collect::<Result<_>>()?;

    let message = ScannableMessage { name, fields };
    let scanner = message.scanner();
    Ok([
        message.impl_scan_message(),
        scanner.type_definition(),
        scanner.scan_event_defn(),
        scanner.impl_scanner_builder(),
        scanner.impl_into_scan(),
        scanner.impl_into_scan_output(),
        scanner.impl_scan_callbacks(),
        scanner.output_type().scan_output_definition(),
    ]
    .into_iter()
    .chain(scanner.fields().map(|m| m.impl_()))
    .flatten()
    .collect())
}

struct ProstAttrs {
    field_type: MessageFieldType,
}

impl TryFrom<(Vec<Attribute>, syn::Type)> for ProstAttrs {
    type Error = syn::Error;

    fn try_from(
        (attributes, rust_field_type): (Vec<Attribute>, syn::Type),
    ) -> std::result::Result<Self, Self::Error> {
        let attrs = prost_attrs(attributes)?;

        enum FieldNumber {
            Single(u32),
            Multiple(Vec<u32>),
        }

        let mut field_type = None;
        let mut field_number = None;
        let mut repeated = false;

        let field_type_names = [
            ("bool", VarintFieldType::Bool.into()),
            ("int32", VarintFieldType::I32.into()),
            ("int64", VarintFieldType::I64.into()),
            ("uint32", VarintFieldType::U32.into()),
            ("uint64", VarintFieldType::U64.into()),
            ("sint32", VarintFieldType::I32Z.into()),
            ("sint64", VarintFieldType::I64Z.into()),
            ("fixed32", FixedFieldType::U32.into()),
            ("fixed64", FixedFieldType::U64.into()),
            ("sfixed32", FixedFieldType::I32.into()),
            ("sfixed64", FixedFieldType::I64.into()),
            ("message", ParsedFieldType::Message),
            ("float", FixedFieldType::F32.into()),
            ("double", FixedFieldType::F64.into()),
            ("bytes", ParsedFieldType::Bytes { utf8: false }),
            ("string", ParsedFieldType::Bytes { utf8: true }),
        ];

        for attr in attrs {
            for (name, found_type) in &field_type_names {
                if attr.path().is_ident(name) {
                    let _ = attr.require_path_only();
                    if let Some(t) = field_type.replace(found_type.clone()) {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("already found type {t:?}"),
                        ));
                    }
                }
            }
            if attr.path().is_ident("oneof") {
                let value = &attr.require_name_value()?.value;
                let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(value),
                    ..
                }) = value
                else {
                    return Err(syn::Error::new(attr.span(), "oneof has a non-path value"));
                };
                if let Some(t) = field_type.replace(ParsedFieldType::OneOf {
                    ty: syn::parse_str(&value.value())?,
                }) {
                    return Err(syn::Error::new(
                        attr.span(),
                        format!("already found type {t:?}"),
                    ));
                }
            }
            if attr.path().is_ident("repeated") {
                let _ = attr.require_path_only();
                repeated = true;
            }
            if attr.path().is_ident("tag") {
                let value = &attr.require_name_value()?.value;
                let value: u32 = match value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(value),
                        ..
                    }) => value.value().parse().ok(),
                    _ => None,
                }
                .ok_or_else(|| {
                    syn::Error::new(
                        attr.span(),
                        format!("unsupported tag value {:?}", value.span().source_text()),
                    )
                })?;
                if field_number.replace(FieldNumber::Single(value)).is_some() {
                    return Err(syn::Error::new(value.span(), "more than one tag number"));
                }
            }
            if attr.path().is_ident("tags") {
                let value = &attr.require_name_value()?.value;
                let values = match value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(value),
                        ..
                    }) => {
                        value
                            .value()
                            .split(',')
                            .map(|v| v.trim().parse())
                            .collect::<std::result::Result<Vec<_>, _>>()
                    }
                    .ok(),
                    _ => None,
                }
                .ok_or_else(|| {
                    syn::Error::new(
                        attr.span(),
                        format!("unsupported tags value {:?}", value.span().source_text()),
                    )
                })?;
                if field_number
                    .replace(FieldNumber::Multiple(values))
                    .is_some()
                {
                    return Err(syn::Error::new(value.span(), "more than one tag number"));
                }
            }
        }

        let field_type = match (field_type, field_number, repeated) {
            (Some(ParsedFieldType::Single(ty)), Some(FieldNumber::Single(number)), true) => {
                MessageFieldType::Repeated(RepeatedField {
                    number,
                    ty: ty.into(),
                })
            }
            (Some(ParsedFieldType::Single(ty)), Some(FieldNumber::Single(number)), false) => {
                MessageFieldType::Single(SingleField { ty, number })
            }
            (Some(ParsedFieldType::Message), Some(FieldNumber::Single(number)), false) => {
                let type_path = strip_outer_path(&rust_field_type)?;
                MessageFieldType::Message(MessageField { number, type_path })
            }
            (Some(ParsedFieldType::Message), Some(FieldNumber::Single(number)), true) => {
                let type_path = strip_outer_path(&rust_field_type)?;
                MessageFieldType::Repeated(RepeatedField {
                    number,
                    ty: RepeatedFieldType::Message { type_path },
                })
            }
            (Some(ParsedFieldType::Bytes { utf8 }), Some(FieldNumber::Single(number)), false) => {
                MessageFieldType::Bytes(BytesField { utf8, number })
            }
            (
                Some(
                    ParsedFieldType::Single(_)
                    | ParsedFieldType::Message
                    | ParsedFieldType::Bytes { .. },
                ),
                Some(FieldNumber::Multiple(_)),
                _,
            ) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "only oneofs support multiple tags",
                ));
            }
            (Some(ParsedFieldType::OneOf { ty }), Some(FieldNumber::Multiple(numbers)), false) => {
                MessageFieldType::OneOf {
                    type_name: ty,
                    numbers,
                }
            }
            (Some(ParsedFieldType::OneOf { .. }), Some(FieldNumber::Single(_)), _) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "oneofs require multiple tags",
                ));
            }
            (Some(ParsedFieldType::OneOf { .. }), _, true) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "oneofs can't be repeated",
                ));
            }
            (Some(ParsedFieldType::Bytes { utf8: _ }), Some(_number), true) => {
                MessageFieldType::Unsupported
            }
            (None, _, _) => MessageFieldType::Unsupported,
            (
                Some(
                    ft @ (ParsedFieldType::Single(_)
                    | ParsedFieldType::Message
                    | ParsedFieldType::Bytes { .. }
                    | ParsedFieldType::OneOf { .. }),
                ),
                None,
                _repeated,
            ) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("no field number for {ft:?}"),
                ));
            }
        };

        Ok(Self { field_type })
    }
}

fn strip_outer_path(ty: &syn::Type) -> Result<syn::TypePath> {
    let span = ty.span();
    fn inner(ty: &syn::Type) -> std::result::Result<syn::TypePath, Cow<'static, str>> {
        let path = match ty {
            syn::Type::Path(path) => path,
            _ => Err("unsupported message field type")?,
        };

        let last = path.path.segments.iter().last().ok_or("path is empty")?;

        if last.ident == "Vec" || last.ident == "Option" {
            match &last.arguments {
                syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                    if let syn::GenericArgument::Type(ty) = args.args.iter().next().unwrap() {
                        return inner(ty);
                    }
                }
                _ => {}
            }
            Err(format!("unrecognized {} type param", last.ident).into())
        } else {
            if last.arguments.is_empty() {
                Ok(path.clone())
            } else {
                Err("unrecognized type is templated")?
            }
        }
    }

    inner(ty).map_err(|e| syn::Error::new(span, e))
}

/// Get the items belonging to the 'prost' list attribute, e.g. `#[prost(foo, bar="baz")]`.
fn prost_attrs(attrs: Vec<Attribute>) -> Result<Vec<Meta>> {
    let mut result = Vec::new();
    for attr in attrs.iter() {
        if let Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("prost") {
                result.extend(
                    meta_list
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                        .into_iter(),
                )
            }
        }
    }
    Ok(result)
}
