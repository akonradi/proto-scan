use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{BytesField, Field, MessageField, OneOfField, SingleField};
use crate::oneof::scanner::OneofScanner;

pub mod scanner;

#[derive(Debug)]
pub struct ScannableOneof {
    pub name: Ident,
    pub fields: Vec<Field<OneOfField>>,
}

impl ScannableOneof {
    pub fn scanner(&self) -> OneofScanner<'_> {
        OneofScanner::new(self)
    }

    pub fn impl_scan_message(&self) -> TokenStream {
        let name = &self.name;
        let scanner_name = self.scanner().type_name();
        let no_op = quote!(::proto_scan::scan::field::NoOp);
        let no_ops = std::iter::repeat_n(&no_op, self.fields.len());
        quote! {
            impl ::proto_scan::scan::ScanMessage for #name {
                type ScannerBuilder = #scanner_name <#(#no_ops),*>;

                fn scanner() -> Self::ScannerBuilder {
                    ::core::default::Default::default()
                }
            }
        }
    }
}

pub struct OneofScannerField<'a> {
    parent: OneofScanner<'a>,
    index: usize,
    field: &'a Field<OneOfField>,
}

impl OneofScannerField<'_> {
    fn scanner_impl_fns(&self) -> Vec<TokenStream> {
        let Self {
            parent,
            index,
            field:
                Field {
                    field_name,
                    field_type,
                    ..
                },
        } = self;
        let scanner_name = parent.type_name();
        let scanner_fields = parent
            .fields()
            .map(|f| &f.field.field_name)
            .collect::<Vec<_>>();
        let (before_no_op, after_no_op) = {
            let mut generic_types = parent.fields().map(|f| &f.field.generic);
            (
                (&mut generic_types).take(*index).collect::<Vec<_>>(),
                generic_types.skip(1).collect::<Vec<_>>(),
            )
        };
        let output_type = self.parent.scan_output_name();

        let swap_single_field_fn =
            |fn_name: Ident,
             docs: Vec<String>,
             generics: Vec<TokenStream>,
             args: Vec<TokenStream>,
             output_type: TokenStream,
             construct_field: TokenStream| {
                quote! {
                    #( #[doc = #docs] )*
                    pub fn #fn_name <#(#generics),*>(
                        self,
                        #(#args),*
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            #output_type,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* proto_scan_last_set } = self;
                        let _ = #field_name;
                        let #field_name = #construct_field;
                        #scanner_name { #(#scanner_fields,)* proto_scan_last_set }
                    }
                }
            };

        let custom_fn = swap_single_field_fn(
            format_ident!("{field_name}"),
            vec![
                format!("Sets the field scanner for oneof field `{field_name}`."),
                "".to_owned(),
                format!(
                    "This allows the caller to specify the behavior on
                    encountering the field `{field_name}` defined in the source
                    oneof. The output of the provided field scanner will be
                    included in the overall scan output as
                    [`{output_type}::{field_name}`]."
                ),
            ],
            vec![
                quote!('t),
                quote!(S: ::proto_scan::scan::IntoScanner<Scanner<::proto_scan::read::BoundsOnlyReadTypes>: ::proto_scan::scan::Resettable> + 't),
            ],
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
            OneOfField::Single(SingleField {
                ty: single,
                number: _,
            }) => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let write_fn = swap_single_field_fn(
                    format_ident!("write_{field_name}"),
                    write_fn_docs(),
                    vec![quote!('t), quote!(D: From<#repr_type>)],
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
                vec![write_fn, save_fn, custom_fn]
            }
            OneOfField::Bytes(BytesField { utf8, number: _ }) => {
                let borrow_type = if *utf8 {
                    quote! {::core::primitive::str}
                } else {
                    quote! {[::core::primitive::u8]}
                };
                let write_fn = swap_single_field_fn(
                    format_ident!("write_{field_name}"),
                    write_fn_docs(),
                    vec![
                        quote!('t),
                        quote!(D: for<'d> ::core::convert::From<&'d #borrow_type>),
                    ],
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
                vec![write_fn, save_fn, custom_fn]
            }
            OneOfField::Message(MessageField {
                number: _,
                type_name,
            }) => {
                let message_name = format_ident!("{type_name}");
                let scan_fn = swap_single_field_fn(
                    format_ident!("scan_{field_name}"),
                    vec![
                        format!("Sets the scanner for the embedded message `{field_name}`."),
                        "".to_owned(),
                        format!(
                            "Sets the builder to use the provided scanner to
                            read the contents of the message in `{field_name}`.
                            The output of the scanner will be included in the
                            overall scan output as
                            [`{output_type}::{field_name}`]."
                        ),
                    ],
                    vec![
                        quote!('t),
                        quote! {
                            S:
                                ::proto_scan::scan::IntoResettable<Resettable:
                                    ::proto_scan::scan::ScannerBuilder<Message=super::#message_name>
                                > + 't
                        },
                    ],
                    vec![quote!(scanner: S)],
                    quote!(
                        ::proto_scan::scan::field::Message<
                            <S as ::proto_scan::scan::IntoResettable>::Resettable,
                        >
                    ),
                    quote!(::proto_scan::scan::field::Message::new(scanner)),
                );
                vec![scan_fn, custom_fn]
            }
        }
    }
}
