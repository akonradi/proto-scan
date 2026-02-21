use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, parse_quote};

#[derive(Debug)]
pub struct MessageField {
    pub field_name: Ident,
    pub generic: Ident,
    pub field_type: FieldType,
}

pub(crate) struct MessageScannerField<'m> {
    pub(crate) parent: super::MessageScanner<'m>,
    pub(crate) index: usize,
    pub(crate) field: &'m MessageField,
}

impl MessageField {
    pub(crate) fn generic(&self) -> &Ident {
        &self.generic
    }
}

impl MessageScannerField<'_> {
    pub fn impl_(&self) -> TokenStream {
        let Self {
            parent,
            index,
            field: _,
        } = self;
        let scanner = parent.0.scanner();
        let scanner_name = scanner.type_name();
        let generic_types = scanner.generic_types().collect::<Vec<_>>();
        let (before_no_op, tail) = generic_types.split_at(*index);
        let (_, after_no_op) = tail.split_first().unwrap();

        let impl_fns = self.scanner_impl_fns();

        quote! {
            impl< #(#before_no_op,)* #(#after_no_op),* > #scanner_name< #(#before_no_op,)* ::proto_scan::scan::field::NoOp, #(#after_no_op),*> {
                #(#impl_fns)*
            }
        }
    }

    fn scanner_impl_fns(&self) -> Vec<TokenStream> {
        let Self {
            parent,
            index,
            field:
                MessageField {
                    field_name,
                    field_type,
                    ..
                },
        } = self;
        let scanner_name = parent.type_name();
        let scanner_fields = parent.field_names().collect::<Vec<_>>();
        let generic_types = parent.generic_types();
        let before_no_op = generic_types.clone().take(*index).collect::<Vec<_>>();
        let after_no_op = generic_types.skip(*index + 1).collect::<Vec<_>>();
        match field_type {
            FieldType::Single { ty: single, number: _ } => {
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
                        let _ = #field_name;
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
                        let _ = #field_name;
                        let #field_name = ::proto_scan::scan::field::EmitScalar::<#encoding_type>::new();
                        #scanner_name { #(#scanner_fields,)* }
                    }
                };
                vec![save_fn, emit_fn]
            }
            FieldType::Unsupported => Vec::<TokenStream>::new(),
        }
    }
}

#[derive(Debug)]
pub enum FieldType {
    Single { ty: SingleFieldType, number: u32 },
    Unsupported,
}

#[derive(Copy, Clone, Debug)]
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
            SingleFieldType::Bool => {
                parse_quote!(::proto_scan::scan::encoding::Varint<::core::primitive::bool>)
            }
            SingleFieldType::FixedU64 => {
                parse_quote!(::proto_scan::scan::encoding::Fixed<::core::primitive::u64>)
            }
        }
    }
}
