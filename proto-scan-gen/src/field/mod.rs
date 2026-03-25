use derive_where::derive_where;
use proc_macro2::TokenStream;
use quote::{ToTokens as _, quote};
use syn::{Ident, parse_quote};

pub mod scanner;

#[derive(Debug)]
pub struct Field<F = MessageFieldType> {
    pub field_name: Ident,
    pub variant_name: Ident,
    pub generic: Ident,
    pub field_type: F,
}

impl<F> Field<F> {
    pub(crate) fn generic(&self) -> FieldGeneric<'_, F> {
        FieldGeneric(self)
    }
}

/// The generic type for a message field.
#[derive_where(Clone, Copy;)]
pub(crate) struct FieldGeneric<'a, F>(&'a Field<F>);

impl<'a, F> FieldGeneric<'a, F> {
    pub(crate) fn ident(self) -> &'a Ident {
        &self.0.generic
    }
}

impl<'a> FieldGeneric<'a, MessageFieldType> {
    pub(crate) fn scan_callbacks_trait_for_bound(&self) -> syn::Path {
        let Self(Field {
            generic: _,
            field_type,
            field_name: _,
            variant_name: _,
        }) = self;
        match field_type {
            MessageFieldType::OneOf {
                numbers: _,
                type_name,
            } => {
                parse_quote!(::proto_scan::scan::ScanCallbacks<R, <#type_name as ::proto_scan::scan::ScannableOneOf>::FieldNumber >)
            }
            MessageFieldType::Single(_)
            | MessageFieldType::Repeated { .. }
            | MessageFieldType::Bytes(_)
            | MessageFieldType::Message(_)
            | MessageFieldType::Group(_)
            | MessageFieldType::Map(_)
            | MessageFieldType::Unsupported => {
                parse_quote!(::proto_scan::scan::field::OnScanField<R>)
            }
        }
    }
}

#[derive(Debug)]
pub struct SingleField {
    pub ty: SingleFieldType,
    pub number: u32,
    pub optional: bool,
}

#[derive(Debug)]
pub struct RepeatedField {
    pub ty: RepeatedFieldType,
    pub number: u32,
}

#[derive(Debug)]
pub struct MessageField {
    pub number: u32,
    pub(crate) type_path: syn::TypePath,
}

impl MessageField {
    pub fn type_name(&self) -> &syn::Ident {
        &self.type_path.path.segments.last().unwrap().ident
    }

    fn as_into_scanner_type(&self) -> TokenStream {
        let Self {
            type_path,
            number: _,
        } = self;
        quote! {::proto_scan::scan::field::Message < #type_path >}
    }
}

#[derive(Debug)]
pub struct BytesField {
    pub utf8: bool,
    pub number: u32,
}
impl BytesField {
    fn as_into_scanner_type(&self) -> TokenStream {
        match self.utf8 {
            false => quote!([u8]),
            true => quote!(str),
        }
    }
}

#[derive(Debug)]
pub enum OneOfField {
    Single(SingleField),
    Bytes(BytesField),
    Message(MessageField),
    Group(GroupField),
}
impl OneOfField {
    pub(crate) fn number(&self) -> u32 {
        match self {
            OneOfField::Single(single_field) => single_field.number,
            OneOfField::Bytes(bytes_field) => bytes_field.number,
            OneOfField::Message(message_field) => message_field.number,
            OneOfField::Group(group_field) => group_field.number,
        }
    }

    pub fn as_into_scanner_type(&self) -> TokenStream {
        match self {
            OneOfField::Single(single_field) => single_field.ty.encoding_type().to_token_stream(),
            OneOfField::Bytes(bytes_field) => bytes_field.as_into_scanner_type(),
            OneOfField::Group(group_field) => group_field.as_into_scanner_type(),
            OneOfField::Message(message_field) => message_field.as_into_scanner_type(),
        }
    }
}
#[derive(Debug)]
pub struct MapField {
    pub number: u32,
    pub key: MapKeyType,
    pub value: MapValueType<syn::TypePath>,
}

#[derive(Debug)]
pub struct GroupField {
    pub number: u32,
    pub type_path: syn::TypePath,
}
impl GroupField {
    fn as_into_scanner_type(&self) -> TokenStream {
        let Self {
            type_path,
            number: _,
        } = self;
        quote! {::proto_scan::scan::field::Group < #type_path >}
    }
}

#[derive(Debug)]
pub enum MessageFieldType {
    Single(SingleField),
    Repeated(RepeatedField),
    Bytes(BytesField),
    Message(MessageField),
    OneOf {
        type_name: syn::Path,
        numbers: Vec<u32>,
    },
    Map(MapField),
    Unsupported,
    Group(GroupField),
}

impl MessageFieldType {
    pub fn as_into_scanner_type(&self) -> TokenStream {
        match self {
            MessageFieldType::Single(single_field) => {
                let single = single_field.ty.encoding_type();
                if single_field.optional {
                    quote!(::core::option::Option<#single>)
                } else {
                    single.to_token_stream()
                }
            }
            MessageFieldType::Repeated(repeated_field) => match &repeated_field.ty {
                RepeatedFieldType::Single(single) => {
                    let inner = single.encoding_type();
                    parse_quote!(::proto_scan::scan::field::Repeated<#inner>)
                }
                RepeatedFieldType::Message { type_path } => {
                    let m = MessageField {
                        type_path: type_path.clone(),
                        number: 0,
                    }
                    .as_into_scanner_type();
                    parse_quote!(::proto_scan::scan::field::Repeated<#m>)
                }
                RepeatedFieldType::Group { type_path } => {
                    let m = GroupField {
                        type_path: type_path.clone(),
                        number: 0,
                    }
                    .as_into_scanner_type();
                    parse_quote!(::proto_scan::scan::field::Repeated<#m>)
                }
            },
            MessageFieldType::Bytes(bytes_field) => bytes_field.as_into_scanner_type(),
            MessageFieldType::Message(message_field) => message_field.as_into_scanner_type(),
            MessageFieldType::Group(group_field) => group_field.as_into_scanner_type(),
            MessageFieldType::OneOf {
                type_name,
                numbers: _,
            } => type_name.into_token_stream(),
            MessageFieldType::Map(MapField {
                key,
                value,
                number: _,
            }) => {
                let key_type = key.encoding_type();
                let value_type = value.encoding_type();
                parse_quote!(::proto_scan::scan::field::Map<#key_type, #value_type>)
            }
            MessageFieldType::Unsupported => quote! { () },
        }
    }
}

#[derive(Copy, Clone, Debug, derive_more::From)]
pub enum SingleFieldType {
    Varint(VarintFieldType),
    Fixed(FixedFieldType),
}

#[derive(Clone, Debug, derive_more::From)]
pub enum RepeatedFieldType {
    #[from]
    Single(SingleFieldType),
    Message {
        type_path: syn::TypePath,
    },
    Group {
        type_path: syn::TypePath,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct MapFieldType {
    pub key: MapKeyType,
    pub value: MapValueType,
}

#[derive(Copy, Clone, Debug, derive_more::From)]
pub enum MapKeyType {
    String,
    #[from(SingleFieldType, VarintFieldType, FixedFieldType)]
    Numeric(SingleFieldType),
}
impl MapKeyType {
    fn encoding_type(&self) -> syn::Path {
        match self {
            MapKeyType::String => parse_quote!(::core::primitive::str),
            MapKeyType::Numeric(single_field_type) => single_field_type.encoding_type(),
        }
    }
}

#[derive(Copy, Clone, Debug, derive_more::From)]
pub enum MapValueType<M = ()> {
    #[from(SingleFieldType, VarintFieldType, FixedFieldType)]
    Single(SingleFieldType),
    Message(M),
    Bytes {
        utf8: bool,
    },
}
impl MapValueType<syn::TypePath> {
    fn encoding_type(&self) -> syn::TypePath {
        match self {
            MapValueType::Single(single_field_type) => syn::TypePath {
                path: single_field_type.encoding_type(),
                qself: None,
            },
            MapValueType::Message(m) => parse_quote!(::proto_scan::scan::field::Message<#m>),
            MapValueType::Bytes { utf8: true } => parse_quote!(::core::primitive::str),
            MapValueType::Bytes { utf8: false } => parse_quote!([::core::primitive::u8]),
        }
    }
}

#[derive(Clone, Debug, derive_more::From)]
pub enum ParsedFieldType {
    #[from(SingleFieldType, VarintFieldType, FixedFieldType)]
    Single(SingleFieldType),
    Message,
    Bytes {
        utf8: bool,
    },
    OneOf {
        ty: syn::Path,
    },
    Map(MapFieldType),
    Group,
}

#[derive(Copy, Clone, Debug)]
pub enum VarintFieldType {
    Bool,
    I32,
    I32Z,
    U32,
    I64,
    U64,
    I64Z,
}

#[derive(Copy, Clone, Debug)]
pub enum FixedFieldType {
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl SingleFieldType {
    pub(crate) fn repr_type(&self) -> syn::Path {
        use SingleFieldType::*;
        match self {
            Varint(VarintFieldType::Bool) => parse_quote!(::core::primitive::bool),
            Varint(VarintFieldType::I32 | VarintFieldType::I32Z) | Fixed(FixedFieldType::I32) => {
                parse_quote!(::core::primitive::i32)
            }
            Varint(VarintFieldType::U32) | Fixed(FixedFieldType::U32) => {
                parse_quote!(::core::primitive::u32)
            }
            Varint(VarintFieldType::I64 | VarintFieldType::I64Z) | Fixed(FixedFieldType::I64) => {
                parse_quote!(::core::primitive::i64)
            }
            Varint(VarintFieldType::U64) | Fixed(FixedFieldType::U64) => {
                parse_quote!(::core::primitive::u64)
            }
            Fixed(FixedFieldType::F32) => {
                parse_quote!(::core::primitive::f32)
            }
            Fixed(FixedFieldType::F64) => {
                parse_quote!(::core::primitive::f64)
            }
        }
    }

    pub(crate) fn encoding_type(&self) -> syn::Path {
        use SingleFieldType::*;
        let repr_type = self.repr_type();
        match self {
            Varint(
                VarintFieldType::Bool
                | VarintFieldType::I32
                | VarintFieldType::U32
                | VarintFieldType::I64
                | VarintFieldType::U64,
            ) => {
                parse_quote!(::proto_scan::scan::encoding::Varint<#repr_type>)
            }
            Varint(VarintFieldType::I32Z | VarintFieldType::I64Z) => {
                parse_quote!(::proto_scan::scan::encoding::Varint<::proto_scan::scan::encoding::ZigZag<#repr_type>>)
            }
            Fixed(_) => {
                parse_quote!(::proto_scan::scan::encoding::Fixed<#repr_type>)
            }
        }
    }
}
