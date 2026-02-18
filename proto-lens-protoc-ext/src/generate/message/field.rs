use proc_macro2::TokenStream;
use prost_types::field_descriptor_proto::Type;
use quote::{format_ident, quote};

use crate::generate::message::{MessageField, ProtoMessage};

pub struct MessageScannerField<'m> {
    pub parent: &'m ProtoMessage,
    pub index: usize,
    pub field: &'m MessageField,
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
            Type::Bool => {
                let save_fn = format_ident!("save_{field_name}");
                let save_fn = quote! {
                    pub fn #save_fn <'t>(
                        self,
                        to: &'t mut impl From<bool>,
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            impl ::proto_lens_scan::OnScanField<ScanEvent = ::core::convert::Infallible> + 't,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let #field_name = ::proto_lens_scan::SaveField::<'_, ::proto_lens_scan::Varint, bool, _>::new(to);
                        #scanner_name { #(#scanner_fields,)* }
                    }
                };
                let emit_fn = format_ident!("emit_{field_name}");
                let emit_fn = quote! {
                    pub fn #emit_fn<'t>(
                        self,
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            impl ::proto_lens_scan::OnScanField<ScanEvent = bool> + 't,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let #field_name = ::proto_lens_scan::EmitScalarField::<::proto_lens_scan::Varint, bool>::new();
                        #scanner_name { #(#scanner_fields,)* }
                    }
                };
                vec![save_fn, emit_fn]
            }
            Type::Double
            | Type::Float
            | Type::Int64
            | Type::Uint64
            | Type::Int32
            | Type::Fixed64
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
            | Type::Sint64 => Vec::<TokenStream>::new(),
        };

        let before_no_op2 = before_no_op.clone();
        let after_no_op2 = after_no_op.clone();

        quote! {
            impl< #(#before_no_op,)* #(#after_no_op),* > #scanner_name< #(#before_no_op2,)* ::proto_lens_scan::NoOp, #(#after_no_op2),*> {
                #(#impl_fns)*
            }
        }
    }
}
