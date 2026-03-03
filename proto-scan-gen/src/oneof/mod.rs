use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{BytesField, Field, MessageField, OneOfField, SingleField};
use crate::oneof::scanner::OneofScanner;
use crate::scanner::{Scanner as _, SwapSingleFieldFn};

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
    inner: crate::scanner::SwapSingleFieldInherentImpl<'a, OneofScanner<'a>, OneOfField>,
}

impl OneofScannerField<'_> {
    pub(crate) fn impl_(&self) -> TokenStream {
        let Self { inner } = self;

        let parent = &inner.parent;
        let field_name = &inner.field.field_name;
        let field_type = &inner.field.field_type;
        let output_type = parent.scan_output_name();

        let custom_fn = SwapSingleFieldFn {
            fn_verb: "",
            docs: &[
                &format!("Sets the field scanner for oneof field `{field_name}`."),
                "",
                &format!(
                    "This allows the caller to specify the behavior on
                    encountering the field `{field_name}` defined in the source
                    oneof. The output of the provided field scanner will be
                    included in the overall scan output as
                    [`{output_type}::{field_name}`]."
                ),
            ],
            generics: &[
                quote!('t),
                quote!(S: ::proto_scan::scan::IntoScanner<Scanner<::proto_scan::read::BoundsOnlyReadTypes>: ::proto_scan::scan::Resettable> + 't),
            ],
            args: &[quote!(scanner: S)],
            output_type: quote!(S),
            construct_field: quote!(scanner),
        };

        match field_type {
            OneOfField::Single(SingleField {
                ty: single,
                number: _,
            }) => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let write_docs = inner.write_fn_docs();
                let write_fn = SwapSingleFieldFn {
                    fn_verb: "write",
                    docs: &write_docs.each_ref().map(|s| &**s),
                    generics: &[quote!('t), quote!(D: From<#repr_type>)],
                    args: &[quote!(to: &'t mut D)],
                    output_type: quote! {::proto_scan::scan::field::WriteNumeric::<#encoding_type, &'t mut D>},
                    construct_field: quote!(::proto_scan::scan::field::WriteNumeric::<#encoding_type, _>::new(to)),
                };
                let save_docs = inner.save_fn_docs();
                let save_fn = SwapSingleFieldFn {
                    fn_verb: "save",
                    docs: &save_docs.each_ref().map(|s| &**s),
                    output_type: quote! {::proto_scan::scan::field::SaveNumeric::<#encoding_type>},
                    construct_field: quote!(::proto_scan::scan::field::SaveNumeric::<#encoding_type>::new()),
                    ..Default::default()
                };
                inner.generate_fns([write_fn, save_fn, custom_fn])
            }
            OneOfField::Bytes(BytesField { utf8, number: _ }) => {
                let borrow_type = if *utf8 {
                    quote! {::core::primitive::str}
                } else {
                    quote! {[::core::primitive::u8]}
                };
                let write_docs = inner.write_fn_docs();
                let write_fn = SwapSingleFieldFn {
                    fn_verb: "write",
                    docs: &write_docs.each_ref().map(|s| &**s),
                    generics: &[
                        quote!('t),
                        quote!(D: for<'d> ::core::convert::From<&'d #borrow_type>),
                    ],
                    args: &[quote!(to: &'t mut D)],
                    output_type: quote! {::proto_scan::scan::field::WriteBytes::<#borrow_type, &'t mut D>},
                    construct_field: quote!(::proto_scan::scan::field::WriteBytes::<#borrow_type, _>::new(to)),
                };
                let save_docs = inner.save_fn_docs();
                let save_fn = SwapSingleFieldFn {
                    fn_verb: "save",
                    docs: &save_docs.each_ref().map(|s| &**s),
                    output_type: quote! {::proto_scan::scan::field::SaveBytes::<#borrow_type>},
                    construct_field: quote!(::proto_scan::scan::field::SaveBytes::<#borrow_type>::new()),
                    ..Default::default()
                };
                inner.generate_fns([write_fn, save_fn, custom_fn])
            }
            OneOfField::Message(MessageField {
                number: _,
                type_name,
            }) => {
                let message_name = format_ident!("{type_name}");
                let docs = &[
                    &format!("Sets the scanner for the embedded message `{field_name}`."),
                    "",
                    &format!(
                        "Sets the builder to use the provided scanner to
                        read the contents of the message in `{field_name}`.
                        The output of the scanner will be included in the
                        overall scan output as
                        [`{output_type}::{field_name}`]."
                    ),
                ];
                let generics = [
                    quote!('t),
                    quote! {
                        S:
                            ::proto_scan::scan::IntoResettable<Resettable:
                                ::proto_scan::scan::ScannerBuilder<Message=super::#message_name>
                            > + 't
                    },
                ];
                let output_type = quote!(
                    ::proto_scan::scan::field::Message<
                        <S as ::proto_scan::scan::IntoResettable>::Resettable,
                    >
                );
                let scan_fn = SwapSingleFieldFn {
                    fn_verb: "scan",
                    docs,
                    generics: &generics,
                    args: &[quote!(scanner: S)],
                    output_type,
                    construct_field: quote!(::proto_scan::scan::field::Message::new(scanner)),
                };
                inner.generate_fns([scan_fn, custom_fn])
            }
        }
    }
}
