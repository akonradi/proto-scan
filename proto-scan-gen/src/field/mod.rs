use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
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

        let swap_single_field_fn =
            |fn_name: Ident,
             generics: Vec<TokenStream>,
             args: Vec<TokenStream>,
             output_type: TokenStream,
             construct_field: TokenStream| {
                quote! {
                    pub fn #fn_name <'t, #(#generics),*>(
                        self,
                        #(#args),*
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            #output_type,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let _ = #field_name;
                        let #field_name = #construct_field;
                        #scanner_name { #(#scanner_fields,)* }
                    }
                }
            };

        let scan_fn = swap_single_field_fn(
            format_ident!("scan_{field_name}"),
            vec![quote!(S: ::proto_scan::scan::field::OnScanField + 't)],
            vec![
                quote!(scanner: S),
            ],
            quote!(S),
            quote!(scanner),
        );

        match field_type {
            FieldType::Single {
                ty: single,
                number: _,
            } => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let save_fn = swap_single_field_fn(
                    format_ident!("save_{field_name}"),
                    vec![quote!(D: From<#repr_type>)],
                    vec![quote!(to: &'t mut D)],
                    quote! {::proto_scan::scan::field::SaveScalar::<'t, #encoding_type, D>},
                    quote!(::proto_scan::scan::field::SaveScalar::<'_, #encoding_type, _>::new(to)),
                );
                let emit_fn = swap_single_field_fn(
                    format_ident!("emit_{field_name}"),
                    vec![],
                    vec![],
                    quote! {::proto_scan::scan::field::EmitScalar::<#encoding_type>},
                    quote!(::proto_scan::scan::field::EmitScalar::<#encoding_type>::new()),
                );
                vec![save_fn, emit_fn, scan_fn]
            }
            FieldType::Repeated { ty, number: _ } => {
                let encoding_type = ty.encoding_type();
                let repr_type = ty.repr_type();

                let save_fn = swap_single_field_fn(
                    format_ident!("save_{field_name}"),
                    vec![quote!(D: ::core::iter::Extend<#repr_type>)],
                    vec![quote!(to: &'t mut D)],
                    quote! {::proto_scan::scan::field::SaveRepeated::<'t, #encoding_type, D>},
                    quote!(::proto_scan::scan::field::SaveRepeated::<'_, #encoding_type, _>::new(to)),
                );
                let emit_fn = swap_single_field_fn(
                    format_ident!("emit_{field_name}"),
                    vec![],
                    vec![],
                    quote! {::proto_scan::scan::field::EmitRepeated::<#encoding_type>},
                    quote!(::proto_scan::scan::field::EmitRepeated::<#encoding_type>::new()),
                );
                vec![save_fn, emit_fn, scan_fn]
            }
            FieldType::Unsupported => vec![scan_fn],
        }
    }
}

#[derive(Debug)]
pub enum FieldType {
    Single { ty: SingleFieldType, number: u32 },
    Repeated { ty: SingleFieldType, number: u32 },
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
