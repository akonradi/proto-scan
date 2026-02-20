use proc_macro2::TokenStream;
use prost_types::FieldDescriptorProto;
use prost_types::field_descriptor_proto::{Label, Type};
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::generate::message::{MessageField, ProtoMessage};

pub struct MessageScannerField<'m> {
    pub parent: &'m ProtoMessage,
    pub index: usize,
    pub field: &'m MessageField,
}

pub enum FieldType {
    Single(SingleFieldType),
    Unsupported,
}

pub enum SingleFieldType {
    Bool,
    FixedU64,
}
impl SingleFieldType {
    
    fn repr_type(&self) -> syn::Path {
        match self {
            SingleFieldType::Bool => parse_quote!(::core::primitive::bool),
            SingleFieldType::FixedU64 => parse_quote!(::core::primitive::u64),
        }
    }
    
    fn encoding_type(&self) -> syn::Path {
        match self {
            SingleFieldType::Bool => parse_quote!(::proto_scan::scan::encoding::Varint<::core::primitive::bool>),
            SingleFieldType::FixedU64 => parse_quote!(::proto_scan::scan::encoding::Fixed<::core::primitive::u64>),
        }
    }
}

impl From<&FieldDescriptorProto> for FieldType {
    fn from(value: &FieldDescriptorProto) -> Self {
        let label = value.label();
        match (value.r#type(), label) {
            (Type::Bool, Label::Optional | Label::Required) => Self::Single(SingleFieldType::Bool),
            (Type::Fixed64, Label::Optional | Label::Required) => Self::Single(SingleFieldType::FixedU64),
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
            ) => Self::Unsupported,
        }
    }
}

impl MessageScannerField<'_> {
    pub fn impl_(&self) -> TokenStream {
        let Self {
            parent,
            index,
            field:
                MessageField {
                    field_name,
                    generic,
                    field_number,
                    field_type,
                },
        } = self;
        let scanner = parent.scanner();
        let scanner_name = scanner.type_name();
        let scanner_fields = scanner.field_names().collect::<Vec<_>>();
        let generic_types = scanner.generic_types();
        let before_no_op = generic_types.clone().take(*index).collect::<Vec<_>>();
        let after_no_op = generic_types.skip(*index + 1).collect::<Vec<_>>();

        let impl_fns = match field_type {
            FieldType::Single(single) => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let save_fn = format_ident!("save_{field_name}");
                let save_fn = quote! {
                    pub fn #save_fn <'t>(
                        self,
                        to: &'t mut impl From<#repr_type>,
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            impl ::proto_scan::scan::field::OnScanField<ScanEvent = ::core::convert::Infallible> + 't,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let #field_name = ::proto_scan::scan::field::Save::<'_, #encoding_type, _>::new(to);
                        #scanner_name { #(#scanner_fields,)* }
                    }
                };
                let emit_fn = format_ident!("emit_{field_name}");
                let emit_fn = quote! {
                    pub fn #emit_fn<'t>(
                        self,
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            impl ::proto_scan::scan::field::OnScanField<ScanEvent = #repr_type> + 't,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let #field_name = ::proto_scan::scan::field::EmitScalar::<#encoding_type>::new();
                        #scanner_name { #(#scanner_fields,)* }
                    }
                };
                vec![save_fn, emit_fn]
            }
            FieldType::Unsupported => Vec::<TokenStream>::new(),
        };

        let before_no_op2 = before_no_op.clone();
        let after_no_op2 = after_no_op.clone();

        quote! {
            impl< #(#before_no_op,)* #(#after_no_op),* > #scanner_name< #(#before_no_op2,)* ::proto_scan::scan::field::NoOp, #(#after_no_op2),*> {
                #(#impl_fns)*
            }
        }
    }
}
