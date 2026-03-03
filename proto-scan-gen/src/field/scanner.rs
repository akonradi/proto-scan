use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::field::{BytesField, Field, MessageField, MessageFieldType, SingleField};
use crate::scanner::{ScannerOutput as _, SwapSingleFieldFn, SwapSingleFieldInherentImpl};

/// A field in a generated message scanner type.
pub(crate) struct MessageScannerField<'m> {
    inner: SwapSingleFieldInherentImpl<
        'm,
        crate::message::scanner::MessageScanner<'m>,
        MessageFieldType,
    >,
}

impl<'m> MessageScannerField<'m> {
    pub(crate) fn new(
        parent: crate::message::scanner::MessageScanner<'m>,
        index: usize,
        field: &'m Field<MessageFieldType>,
    ) -> Self {
        Self {
            inner: SwapSingleFieldInherentImpl {
                parent,
                index,
                field,
            },
        }
    }

    pub fn impl_(&self) -> TokenStream {
        let Self { inner } = self;
        let field_name = &inner.field.field_name;
        let field_type = &inner.field.field_type;
        let output_type = self.inner.parent.scanner().output_type().type_name();
        let custom_fn = SwapSingleFieldFn {
            fn_verb: "",
            docs: vec![
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
            generics: vec![quote!('t), quote!(S: ::proto_scan::scan::IntoScanner + 't)],
            args: vec![quote!(scanner: S)],
            output_type: quote!(S),
            construct_field: quote!(scanner),
        };

        match field_type {
            MessageFieldType::Single(SingleField {
                ty: single,
                number: _,
            }) => {
                let encoding_type = single.encoding_type();
                let repr_type = single.repr_type();

                let write_fn = SwapSingleFieldFn {
                    fn_verb: "write",
                    docs: inner.write_fn_docs(),
                    generics: vec![quote!('t), quote!(D: From<#repr_type>)],
                    args: vec![quote!(to: &'t mut D)],
                    output_type: quote! {::proto_scan::scan::field::WriteNumeric::<#encoding_type, &'t mut D>},
                    construct_field: quote!(::proto_scan::scan::field::WriteNumeric::<#encoding_type, _>::new(to)),
                };
                let save_fn = SwapSingleFieldFn {
                    fn_verb: "save",
                    docs: inner.save_fn_docs(),
                    output_type: quote! {::proto_scan::scan::field::SaveNumeric::<#encoding_type>},
                    construct_field: quote!(::proto_scan::scan::field::SaveNumeric::<#encoding_type>::new()),
                    ..Default::default()
                };
                inner.generate_fns([write_fn, save_fn, custom_fn])
            }
            MessageFieldType::Repeated { ty, number: _ } => {
                let encoding_type = ty.encoding_type();
                let repr_type = ty.repr_type();

                let write_fn = SwapSingleFieldFn {
                    fn_verb: "write",
                    docs: inner.write_fn_docs(),
                    generics: vec![quote!('t), quote!(D: ::core::iter::Extend<#repr_type>)],
                    args: vec![quote!(to: &'t mut D)],
                    output_type: quote! {::proto_scan::scan::field::WriteRepeated::<#encoding_type, &'t mut D>},
                    construct_field: quote!(::proto_scan::scan::field::WriteRepeated::<#encoding_type, _>::new(to)),
                };
                let save_fn = SwapSingleFieldFn {
                    fn_verb: "save",
                    docs: inner.save_fn_docs(),
                    output_type: quote! {::proto_scan::scan::field::SaveRepeated::<#encoding_type>},
                    construct_field: quote!(::proto_scan::scan::field::SaveRepeated::<#encoding_type>::new()),
                    ..Default::default()
                };
                inner.generate_fns([write_fn, save_fn, custom_fn])
            }
            MessageFieldType::Bytes(BytesField { utf8, number: _ }) => {
                let borrow_type = if *utf8 {
                    quote! {::core::primitive::str}
                } else {
                    quote! {[::core::primitive::u8]}
                };
                let write_fn = {
                    SwapSingleFieldFn {
                        fn_verb: "write",
                        docs: inner.write_fn_docs(),
                        generics: vec![
                            quote!('t),
                            quote!(D: for<'d> ::core::convert::From<&'d #borrow_type>),
                        ],
                        args: vec![quote!(to: &'t mut D)],
                        output_type: quote! {::proto_scan::scan::field::WriteBytes::<#borrow_type, &'t mut D>},
                        construct_field: quote!(::proto_scan::scan::field::WriteBytes::<#borrow_type, _>::new(to)),
                    }
                };
                let save_fn = SwapSingleFieldFn {
                    fn_verb: "save",
                    docs: inner.save_fn_docs(),
                    output_type: quote! {::proto_scan::scan::field::SaveBytes::<#borrow_type>},
                    construct_field: quote!(::proto_scan::scan::field::SaveBytes::<#borrow_type>::new()),
                    ..Default::default()
                };
                inner.generate_fns([write_fn, save_fn, custom_fn])
            }
            MessageFieldType::Message(MessageField {
                number: _,
                type_name,
            }) => {
                let message_name = format_ident!("{type_name}");
                let docs = vec![
                    format!("Sets the scanner for the embedded message `{field_name}`."),
                    "".to_owned(),
                    format!(
                        "Sets the builder to use the provided scanner to
                        read the contents of the message in `{field_name}`.
                        The output of the scanner will be included in the
                        overall scan output as
                        [`{output_type}::{field_name}`]."
                    ),
                ];
                let generics = vec![
                    quote!('t),
                    quote! {
                        S:
                            ::proto_scan::scan::IntoResettable<Resettable:
                                ::proto_scan::scan::ScannerBuilder<Message=#message_name>
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
                    generics,
                    args: vec![quote!(scanner: S)],
                    output_type,
                    construct_field: quote!(::proto_scan::scan::field::Message::new(scanner)),
                };
                inner.generate_fns([scan_fn, custom_fn])
            }
            MessageFieldType::OneOf {
                type_name: _,
                numbers: _,
            } => {
                let f = SwapSingleFieldFn {
                    fn_verb: "",
                    docs: vec![
                        format!("Sets the field scanner for the oneof `{field_name}`."),
                        "".to_owned(),
                        format!(
                            "This allows the caller to specify the behavior on
                    encountering any of the fields in the oneof `{field_name}`
                    defined in the source message. The output of the provided
                    field scanner will be included in the overall scan output as
                    [`{output_type}::{field_name}`]."
                        ),
                    ],
                    generics: vec![quote!('t), quote!(S: ::proto_scan::scan::IntoScanner + 't)],
                    args: vec![quote!(scanner: S)],
                    output_type: quote!(S),
                    construct_field: quote!(scanner),
                };
                inner.generate_fns([f])
            }
            MessageFieldType::Unsupported => TokenStream::new(),
        }
    }
}
