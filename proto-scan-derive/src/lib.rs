use proc_macro2::{Span, TokenStream};
use proto_scan_gen::ScannableMessage;
use proto_scan_gen::field::{
    FieldType, FixedFieldType, MessageField, SingleFieldType, VarintFieldType,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, DataStruct, DeriveInput, Ident, Meta, Result, Token};

#[proc_macro_derive(ScanMessage)]
pub fn scan_message_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    derive_impl(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

fn derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = input;

    if !generics.params.is_empty() {
        return Err(syn::Error::new(
            generics.span(),
            "generics are not supported",
        ));
    }

    match data {
        syn::Data::Struct(data_struct) => message_impl(ident, data_struct),
        syn::Data::Enum(data_enum) => todo!(),
        syn::Data::Union(u) => {
            return Err(syn::Error::new(
                u.union_token.span(),
                "union types are not supported",
            ));
        }
    }
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
                vis,
                mutability,
                ident: field_name,
                colon_token,
                ty,
            } = field;

            let field_name =
                field_name.ok_or_else(|| syn::Error::new(span, "message fields must be named"))?;

            let ProstAttrs { field_type } = (span, attrs).try_into()?;

            let generic = Ident::new(&format!("T{i}"), Span::call_site());

            Ok(MessageField {
                field_name,
                field_type,
                generic,
            })
        })
        .collect::<Result<_>>()?;

    let message = ScannableMessage { name, fields };
    Ok([message.scanner().generated_code()].into_iter().collect())
}

struct ProstAttrs {
    field_type: FieldType,
}

impl TryFrom<(Span, Vec<Attribute>)> for ProstAttrs {
    type Error = syn::Error;

    fn try_from((span, value): (Span, Vec<Attribute>)) -> std::result::Result<Self, Self::Error> {
        let attrs = prost_attrs(value)?;

        #[derive(Clone, Copy, Debug, derive_more::From)]
        enum ParsedFieldType {
            #[from(SingleFieldType, VarintFieldType, FixedFieldType)]
            Single(SingleFieldType),
            Message,
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
        ];

        for attr in attrs {
            for (name, found_type) in &field_type_names {
                if attr.path().is_ident(name) {
                    let _ = attr.require_path_only();
                    if let Some(t) = field_type.replace(*found_type) {
                        return Err(syn::Error::new(attr.span(), format!("already found type {t:?}")));
                    }
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
                if let Some(_) = field_number.replace(value) {
                    return Err(syn::Error::new(value.span(), "more than one tag number"));
                }
            }
        }

        let field_type = match (field_type, field_number) {
            (Some(ParsedFieldType::Single(ty)), Some(number)) => {
                if repeated {
                    FieldType::Repeated { ty, number }
                } else {
                    FieldType::Single { ty, number }
                }
            }
            (Some(ParsedFieldType::Message), Some(number)) => FieldType::Message { number },
            _ => FieldType::Unsupported,
        };

        Ok(Self { field_type })
    }
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
