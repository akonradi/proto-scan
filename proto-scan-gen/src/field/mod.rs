use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, parse_quote};

use crate::MessageScanOutput;

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
        let (before_no_op, after_no_op) = {
            let mut generic_types = parent.generic_types();
            (
                (&mut generic_types).take(*index).collect::<Vec<_>>(),
                generic_types.skip(1).collect::<Vec<_>>(),
            )
        };
        let output_type = MessageScanOutput(self.parent).type_name();

        let swap_single_field_fn =
            |fn_name: Ident,
             docs: Vec<String>,
             generics: Vec<TokenStream>,
             args: Vec<TokenStream>,
             output_type: TokenStream,
             construct_field: TokenStream| {
                quote! {
                    #( #[doc = #docs] )*
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
            format_ident!("{field_name}"),
            vec![
                format!("Sets the field scanner for message field `{field_name}`."),
                "".to_owned(),
                format!(
                    "This allows the caller to specify the behavior on
                    encountering the field `{field_name}` defined in the source
                    message. The output of the provided field scanner will be
                    included in the overall scan output as
                    [`{output_type}::{field_name}`]."
                ),
            ],
            vec![quote!(S: ::proto_scan::scan::field::OnScanField + 't)],
            vec![quote!(scanner: S)],
            quote!(S),
            quote!(scanner),
        );

        let write_fn_docs = || {
            vec![
                format!("Sets the scanner to write field `{field_name}` to the provided location."),
                "".to_string(),
                format!(
                    "When the field `{field_name}` is encountered in the input,
                    the decoded value will be written to the argument `to`. No
                    output is provided in the overall scan output
                    ([`{output_type}::{field_name}`] is `()`)."
                ),
            ]
        };
        let save_fn_docs = || {
            vec![
                format!("Sets the scanner to output field `{field_name}`."),
                "".to_owned(),
                format!(
                    "When the field `{field_name}` is encountered in the input
                    during a scan, the decoded value will be saved and produced
                    in the output as [`{output_type}::{field_name}`]."
                ),
            ]
        };

        match field_type {
            FieldType::Single {
                ty: single,
                number: _,
            } => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let write_fn = swap_single_field_fn(
                    format_ident!("write_{field_name}"),
                    write_fn_docs(),
                    vec![quote!(D: From<#repr_type>)],
                    vec![quote!(to: &'t mut D)],
                    quote! {::proto_scan::scan::field::WriteNumeric::<#encoding_type, &'t mut D>},
                    quote!(::proto_scan::scan::field::WriteNumeric::<#encoding_type, _>::new(to)),
                );
                let save_fn = swap_single_field_fn(
                    format_ident!("save_{field_name}"),
                    save_fn_docs(),
                    vec![],
                    vec![],
                    quote! {::proto_scan::scan::field::SaveNumeric::<#encoding_type>},
                    quote!(::proto_scan::scan::field::SaveNumeric::<#encoding_type>::new()),
                );
                vec![write_fn, save_fn, scan_fn]
            }
            FieldType::Repeated { ty, number: _ } => {
                let encoding_type = ty.encoding_type();
                let repr_type = ty.repr_type();

                let write_fn = swap_single_field_fn(
                    format_ident!("write_{field_name}"),
                    write_fn_docs(),
                    vec![quote!(D: ::core::iter::Extend<#repr_type>)],
                    vec![quote!(to: &'t mut D)],
                    quote! {::proto_scan::scan::field::WriteRepeated::<#encoding_type, &'t mut D>},
                    quote!(::proto_scan::scan::field::WriteRepeated::<#encoding_type, _>::new(to)),
                );
                let save_fn = swap_single_field_fn(
                    format_ident!("save_{field_name}"),
                    save_fn_docs(),
                    vec![],
                    vec![],
                    quote! {::proto_scan::scan::field::SaveRepeated::<#encoding_type>},
                    quote!(::proto_scan::scan::field::SaveRepeated::<#encoding_type>::new()),
                );
                vec![write_fn, save_fn, scan_fn]
            }
            FieldType::Bytes { utf8, number: _ } => {
                let borrow_type = if *utf8 {
                    quote! {::core::primitive::str}
                } else {
                    quote! {[::core::primitive::u8]}
                };
                let write_fn = swap_single_field_fn(
                    format_ident!("write_{field_name}"),
                    write_fn_docs(),
                    vec![quote!(D: for<'d> ::core::convert::From<&'d #borrow_type>)],
                    vec![quote!(to: &'t mut D)],
                    quote! {::proto_scan::scan::field::WriteBytes::<#borrow_type, &'t mut D>},
                    quote!(::proto_scan::scan::field::WriteBytes::<#borrow_type, _>::new(to)),
                );
                let save_fn = swap_single_field_fn(
                    format_ident!("save_{field_name}"),
                    save_fn_docs(),
                    vec![],
                    vec![],
                    quote! {::proto_scan::scan::field::SaveBytes::<#borrow_type>},
                    quote!(::proto_scan::scan::field::SaveBytes::<#borrow_type>::new()),
                );
                vec![write_fn, save_fn, scan_fn]
            }
            FieldType::Message { number: _ } => vec![scan_fn],
            FieldType::Unsupported => vec![],
        }
    }
}

#[derive(Debug)]
pub enum FieldType {
    Single { ty: SingleFieldType, number: u32 },
    Repeated { ty: SingleFieldType, number: u32 },
    Bytes { utf8: bool, number: u32 },
    Message { number: u32 },
    Unsupported,
}

#[derive(Copy, Clone, Debug, derive_more::From)]
pub enum SingleFieldType {
    Varint(VarintFieldType),
    Fixed(FixedFieldType),
}

#[derive(Clone, Copy, Debug, derive_more::From)]
pub enum ParsedFieldType {
    #[from(SingleFieldType, VarintFieldType, FixedFieldType)]
    Single(SingleFieldType),
    Message,
    Bytes {
        utf8: bool,
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
    fn repr_type(&self) -> syn::Path {
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

    fn encoding_type(&self) -> syn::Path {
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
