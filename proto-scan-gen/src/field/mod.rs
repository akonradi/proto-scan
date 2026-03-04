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
pub(crate) struct FieldGeneric<'a, F>(&'a Field<F>);

impl<F> Clone for FieldGeneric<'_, F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<F> Copy for FieldGeneric<'_, F> {}

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
                parse_quote!(::proto_scan::scan::OnScanOneof<R, <#type_name as ::proto_scan::scan::ScannableOneOf>::FieldNumber >)
            }
            MessageFieldType::Single(_)
            | MessageFieldType::Repeated { .. }
            | MessageFieldType::Bytes(_)
            | MessageFieldType::Message(_)
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
}

#[derive(Debug)]
pub struct MessageField {
    pub type_name: String,
    pub number: u32,
}

#[derive(Debug)]
pub struct BytesField {
    pub utf8: bool,
    pub number: u32,
}

#[derive(Debug)]
pub enum OneOfField {
    Single(SingleField),
    Bytes(BytesField),
    Message(MessageField),
}
impl OneOfField {
    pub(crate) fn number(&self) -> u32 {
        match self {
            OneOfField::Single(single_field) => single_field.number,
            OneOfField::Bytes(bytes_field) => bytes_field.number,
            OneOfField::Message(message_field) => message_field.number,
        }
    }
}

#[derive(Debug)]
pub enum MessageFieldType {
    Single(SingleField),
    Repeated {
        ty: SingleFieldType,
        number: u32,
    },
    Bytes(BytesField),
    Message(MessageField),
    OneOf {
        type_name: syn::Path,
        numbers: Vec<u32>,
    },
    Unsupported,
}

#[derive(Copy, Clone, Debug, derive_more::From)]
pub enum SingleFieldType {
    Varint(VarintFieldType),
    Fixed(FixedFieldType),
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
