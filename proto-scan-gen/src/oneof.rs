use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{self, BytesField, Field, MessageField, OneOfField, SingleField};

#[derive(Debug)]
pub struct ScannableOneof {
    pub name: Ident,
    pub fields: Vec<Field<OneOfField>>,
}

impl ScannableOneof {
    pub fn scanner(&self) -> OneofScanner<'_> {
        OneofScanner(self)
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

#[derive(Copy, Clone, Debug)]
pub struct OneofScanner<'a>(&'a ScannableOneof);

pub struct OneofScannerField<'a> {
    parent: OneofScanner<'a>,
    index: usize,
    field: &'a Field<OneOfField>,
}

impl<'a> OneofScanner<'a> {
    fn type_name(&self) -> Ident {
        format_ident!("Scan{}", self.0.name)
    }

    pub fn fields(&self) -> impl Iterator<Item = OneofScannerField<'_>> {
        self.0
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| OneofScannerField {
                parent: *self,
                index: i,
                field: f,
            })
    }

    fn scan_event_name(&self) -> Ident {
        let scanner_name = self.type_name();
        Ident::new(&format!("{scanner_name}Event"), Span::call_site())
    }

    pub fn generated_code(&self) -> TokenStream {
        [
            self.scanner_type_definition(),
            self.output_type_definition(),
            self.0.impl_scan_message(),
            self.impl_scanner_builder(),
            self.event_type_definition(),
            self.impl_scan_callbacks(),
            self.scanner_impl_fns(),
        ]
        .into_iter()
        .collect()
    }

    fn scanner_type_definition(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.fields().map(|f| &f.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.field.field_name;
            let generic = &f.field.generic;
            quote! {
                #name: #generic,
            }
        });
        let last_set_type = self.scan_output_name();
        quote! {
            #[derive(Debug, Default)]
            pub struct #type_name<#(#generics),*> {
                #(#fields)*
                proto_scan_last_set: ::core::option::Option<#last_set_type>,
            }
        }
    }

    fn output_type_definition(&self) -> TokenStream {
        let type_name = self.scan_output_name();
        let generics = self.fields().map(|f| &f.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.field.field_name;
            let generic = &f.field.generic;
            quote! {
                #name(#generic)
            }
        });
        quote! {
            #[derive(Copy, Clone, Debug, Hash, PartialEq)]
            pub enum #type_name<#(#generics = ()),*> {
                #(#fields),*
            }
        }
    }

    fn impl_scanner_builder(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.fields().map(|f| &f.field.generic).collect::<Vec<_>>();
        let oneof_type_name = &self.0.name;
        let fields = self.fields().map(|f| &f.field.field_name).collect::<Vec<_>>();
        quote! {
            impl<#(#generics,)*> ::proto_scan::scan::ScannerBuilder for #type_name<#(#generics),*> {
                type Message = #oneof_type_name;
            }

            impl<#(#generics: ::proto_scan::scan::IntoScanner,)*> ::proto_scan::scan::IntoScanner for #type_name<#(#generics),*> {
                type Scanner<R: ::proto_scan::read::ReadTypes> = #type_name<#(#generics::Scanner<R>),*>;

                fn into_scanner<R: ::proto_scan::read::ReadTypes>(self) -> Self::Scanner<R> {
                    let Self { #(#fields,)* proto_scan_last_set } = self;
                    Self::Scanner { #(#fields: #fields.into_scanner(),)* proto_scan_last_set}
                }
            }
        }
    }

    fn event_type_definition(&self) -> TokenStream {
        let type_name = self.scan_event_name();
        let generics = self.fields().map(|f| &f.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.field.field_name;
            let generic = &f.field.generic;
            quote! {
                #name(#generic)
            }
        });
        quote! {
            pub enum #type_name<#(#generics = ()),*> {
                #(#fields),*
            }
        }
    }

    fn impl_scan_callbacks(&self) -> TokenStream {
        let type_name = self.type_name();
        let output_type = self.scan_output_name();
        let fields = self.fields().collect::<Vec<_>>();
        let field_names = fields
            .iter()
            .map(|f| &f.field.field_name)
            .collect::<Vec<_>>();
        let generics = fields.iter().map(|f| &f.field.generic).collect::<Vec<_>>();
        let generics_with_bounds = fields.iter().map(
            |OneofScannerField {
                 parent,
                 index,
                 field:
                     Field {
                         field_name,
                         generic,
                         field_type,
                     },
             }| {
                quote! { #generic: ::proto_scan::scan::field::OnScanField<R> + ::proto_scan::scan::Resettable }
            },
        );
        let last_set_arms = fields.iter().map(|OneofScannerField {parent, index, field: Field { field_name, generic, field_type } }| {
            quote! {
                #output_type::#field_name(()) => #output_type::#field_name(#field_name.into_scan_output())
            }
        });

        let scan_event_name = self.scan_event_name();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let output_type = &output_type;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |OneofScannerField { parent, index, field: Field { field_name, generic: _, field_type } } | {
                let event_variant_name = field_name;
                match field_type {
                    OneOfField::Single(SingleField { ty: _, number })
                    | OneOfField::Message(MessageField{ number, type_name: _ })
                    | OneOfField::Bytes(BytesField{ utf8: _, number }) => quote! {
                        #number => {
                            ::proto_scan::scan::Resettable::reset(self);
                            let event = self.#field_name.#fn_name(value)?;
                            self.proto_scan_last_set = ::core::option::Option::Some(#output_type::#event_variant_name(()));
                            event.map(#scan_event_name::#event_variant_name)
                        },
                    },
                }
            })
        };

        let on_numeric_arms = field_arms("on_numeric");
        let on_group_arms = field_arms("on_group");
        let on_length_delimited_arms = field_arms("on_length_delimited");
        quote! {
            impl<
                #(#generics_with_bounds,)*
                R: ::proto_scan::read::ReadTypes
            > ::proto_scan::scan::OnScanOneof<R> for #type_name< #(#generics),* > {
                type ScanEvent = #scan_event_name < #(<#generics as ::proto_scan::scan::field::OnScanField<R>>::ScanEvent),* >;

                fn on_numeric(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: ::proto_scan::wire::NumericField,
                ) -> Result<::core::option::Option<Self::ScanEvent>, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_numeric_arms)*
                        _ => None,
                    })
                }

                fn on_group(&mut self, field: ::proto_scan::wire::FieldNumber, value: ::proto_scan::wire::GroupOp) -> Result<::core::option::Option<Self::ScanEvent>, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_group_arms)*
                        _ => None,
                    })
                }

                fn on_length_delimited(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: impl ::proto_scan::wire::LengthDelimited<ReadTypes=R>,
                ) -> Result<::core::option::Option<Self::ScanEvent>, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_length_delimited_arms)*
                        _ => None,
                    })
                }
            }
            impl<
                #(#generics: ::proto_scan::scan::IntoScanOutput),*
            > ::proto_scan::scan::IntoScanOutput for #type_name< #(#generics),* > {
                type ScanOutput = ::core::option::Option<#output_type < #(<#generics as ::proto_scan::scan::IntoScanOutput>::ScanOutput),* >>;

                fn into_scan_output(self) -> Self::ScanOutput {
                    let Self { #(#field_names, )* proto_scan_last_set } = self;
                    Some(match proto_scan_last_set? {
                        #(#last_set_arms),*
                    })
                }
            }

            impl<
                #(#generics: ::proto_scan::scan::Resettable),*
            > ::proto_scan::scan::Resettable for #type_name< #(#generics),* > {
                fn reset(&mut self) {
                    let Self { #(#field_names, )* proto_scan_last_set } = self;
                    #(#field_names.reset();)*
                    *proto_scan_last_set = None;
                }
            }

        }
    }

    fn scan_output_name(&self) -> Ident {
        format_ident!("{}Output", self.type_name())
    }

    fn scanner_impl_fns(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let generic_types = self.0.fields.iter().map(|f| &f.generic).collect::<Vec<_>>();

        self.fields().map(|field|{
            let index = field.index;
            let (before_no_op, tail) = generic_types.split_at(index);
            let (_, after_no_op) = tail.split_first().unwrap();

            let impl_fns = field.scanner_impl_fns();

            quote! {
                impl< #(#before_no_op,)* #(#after_no_op),* > #scanner_name< #(#before_no_op,)* ::proto_scan::scan::field::NoOp, #(#after_no_op),*> {
                    #(#impl_fns)*
                }
            }
        }).collect()
    }
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
                    pub fn #fn_name <'t, #(#generics),*>(
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
                    vec![quote! {
                        S:
                            ::proto_scan::scan::IntoResettable<Resettable:
                                ::proto_scan::scan::ScannerBuilder<Message=super::#message_name>
                            > + 't
                    }],
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
