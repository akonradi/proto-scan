use std::borrow::Cow;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::scanner::MessageScannerField;
use crate::field::{
    BytesField, Field, FieldGeneric, MapField, MessageField, MessageFieldType, RepeatedField,
    SingleField,
};
use crate::message::ScannableMessage;
use crate::scanner::{Scanner as _, ScannerOutput as _};

/// Generates the scanner for the inner message.
#[derive(Copy, Clone)]
pub(crate) struct MessageScanner<'m>(&'m ScannableMessage);

impl<'m> MessageScanner<'m> {
    pub fn new(msg: &'m ScannableMessage) -> Self {
        Self(msg)
    }

    pub fn fields(&self) -> impl Iterator<Item = MessageScannerField<'_>> + Clone {
        self.0
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| MessageScannerField::new(*self, index, field))
    }

    pub fn type_definition(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let scan_types = self.generic_types().map(|f| f.ident());
        let scan_fields = self.0.fields.iter().map(
            |Field {
                 field_name,
                 generic,
                 ..
             }| quote!(#field_name: #generic),
        );
        quote! {
            #[derive(Clone, Default)]
            pub struct #scanner_name <#(#scan_types),*> {
                #(#scan_fields, )*
            }
        }
    }

    pub fn scan_event_defn(&self) -> TokenStream {
        let scan_event_name = self.scan_event_name();
        let variants = self.fields().map(|f| {
            let name = f.variant_name();
            let generic = f.generic();
            quote!(#name(#generic))
        });
        let generics = self.generic_types().map(|f| f.ident());
        quote! {
            #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
            pub enum #scan_event_name<#(#generics,)*> {
                #(#variants),*
            }
        }
    }

    pub fn impl_scanner_builder(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self
            .generic_types()
            .map(FieldGeneric::ident)
            .collect::<Vec<_>>();
        let generics_with_bounds = self.0.fields.iter().map(|f| {
            let g = &f.generic;
            let t = f.field_type.as_into_scanner_type();
            quote!(#g: ::proto_scan::scan::IntoScanner<#t>)
        });

        let message_name = &self.0.name;
        quote! {
            impl<#(#generics_with_bounds ),*> ::proto_scan::scan::MessageScanner for #type_name < #(#generics),* > {
                type Message = #message_name;
            }
        }
    }

    pub fn impl_into_scan(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self
            .generic_types()
            .map(FieldGeneric::ident)
            .collect::<Vec<_>>();
        let message_type = &self.0.name;
        let field_names = self.field_names().collect::<Vec<_>>();
        let generics_with_bounds = self.0.fields.iter().map(|f| {
            let g = &f.generic;
            let t = f.field_type.as_into_scanner_type();
            quote!(#g: ::proto_scan::scan::IntoScanner<#t>)
        });
        quote! {
            impl<#(#generics_with_bounds),*> ::proto_scan::scan::IntoScanner<#message_type> for #type_name < #(#generics),* > {
                type Scanner<R: ::proto_scan::read::ReadTypes> = #type_name < #(#generics ::Scanner<R> ),* >;
                fn into_scanner<R: ::proto_scan::read::ReadTypes>(self) -> Self::Scanner<R> {
                    let Self { #(#field_names),* } = self;
                    #type_name {
                        #(#field_names: #field_names.into_scanner()),*
                    }
                }
            }
        }
    }

    pub fn impl_into_scan_output(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let output_name = self.output_type().type_name();
        let field_names = self.field_names().collect::<Vec<_>>();
        let generics = self
            .generic_types()
            .map(FieldGeneric::ident)
            .collect::<Vec<_>>();
        quote! {
            impl <#(#generics: ::proto_scan::scan::IntoScanOutput,)*> ::proto_scan::scan::IntoScanOutput for #scanner_name<#(#generics,)*> {
                type ScanOutput = #output_name<#(#generics::ScanOutput),*>;

                fn into_scan_output(self) -> Self::ScanOutput {
                    let Self { #(#field_names),* } = self;
                    Self::ScanOutput {
                        #(#field_names: #field_names.into_scan_output(),)*
                    }
                }
            }
        }
    }

    pub fn impl_scan_callbacks(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let scan_event_name = self.scan_event_name();
        let generics = self
            .generic_types()
            .map(FieldGeneric::ident)
            .collect::<Vec<_>>();
        let generics_on_scan_bounds = self.generic_types().map(|g| {
            let (ident, bound) = (g.ident(), g.scan_callbacks_trait_for_bound());
            quote!(#ident: #bound)
        });
        let field_names = self.field_names().collect::<Vec<_>>();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let fn_name = format_ident!("{fn_name}");
            self.0.fields.iter()
                .map(
                    move |Field {
                              field_name,
                              generic: _,
                              field_type,
                              variant_name,
                          }| {
                        match field_type {
                            MessageFieldType::Single(SingleField { ty: _, number })
                            | MessageFieldType::Repeated(RepeatedField {ty: _, number})
                            | MessageFieldType::Message(MessageField {
                                number,
                                type_path: _,
                            })
                            | MessageFieldType::Map(MapField {number, key: _, value: _})
                            | MessageFieldType::Bytes(BytesField { utf8: _, number }) => quote! {
                                #number => self.#field_name.#fn_name(value)?.map(#scan_event_name::#variant_name),
                            },
                            MessageFieldType::OneOf {
                                type_name,
                                numbers,
                            } => numbers.iter().map(|number| {
                                quote! {
                                    #number => {
                                        let field_number = <#type_name as ::proto_scan::scan::ScannableOneOf>::FieldNumber::for_field_number::<#number>();
                                        let event = self.#field_name.#fn_name(field_number, value)?;
                                        ::core::option::Option::Some(#scan_event_name::#variant_name(event))
                                    },
                                }
                            }).collect(),
                            MessageFieldType::Unsupported => TokenStream::new(),
                        }
                    },
                )
        };

        let on_numeric_arms = field_arms("on_numeric");
        let on_group_arms = field_arms("on_group");
        let on_length_delimited_arms = field_arms("on_length_delimited");
        quote! {

            impl <#(#generics_on_scan_bounds,)* R: ::proto_scan::read::ReadTypes> ::proto_scan::scan::ScanCallbacks<R> for #scanner_name<#(#generics,)*> {
                type ScanEvent = Option<#scan_event_name<#(#generics :: ScanEvent),*>>;
                fn on_numeric(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: ::proto_scan::wire::NumericField,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::ScanError<R::Error>> {
                    #[allow(clippy::match_single_binding)]
                    Ok(match u32::from(field) {
                        #(#on_numeric_arms)*
                        _ => None,
                    })
                }

                fn on_group(&mut self, field: ::proto_scan::wire::FieldNumber, value: ::proto_scan::wire::GroupOp) -> Result<Self::ScanEvent, ::proto_scan::scan::ScanError<R::Error>> {
                    #[allow(clippy::match_single_binding)]
                    Ok(match u32::from(field) {
                        #(#on_group_arms)*
                        _ => None,
                    })
                }

                fn on_length_delimited(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: impl ::proto_scan::wire::LengthDelimited<ReadTypes=R>,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::ScanError<R::Error>> {
                    #[allow(clippy::match_single_binding)]
                    Ok(match u32::from(field) {
                        #(#on_length_delimited_arms)*
                        _ => None,
                    })
                }
            }
            impl <#(#generics: ::proto_scan::scan::IntoResettable),*> ::proto_scan::scan::IntoResettable for #scanner_name<#(#generics,)*> {
                type Resettable = #scanner_name<#(<#generics as ::proto_scan::scan::IntoResettable>::Resettable),*>;

                fn into_resettable(self) -> Self::Resettable {
                    let Self { #(#field_names),* } = self;
                    #scanner_name {
                        #(#field_names: #field_names.into_resettable()),*
                    }
                }
            }

            impl <#(#generics: ::proto_scan::scan::Resettable),*> ::proto_scan::scan::Resettable for #scanner_name<#(#generics,)*> {
                fn reset(&mut self) {
                    #(self.#field_names.reset();)*
                }
            }
        }
    }

    fn scan_event_name(&self) -> Ident {
        let scanner_name = self.type_name();
        Ident::new(&format!("{scanner_name}Event"), Span::call_site())
    }

    pub(crate) fn scanner(&self) -> MessageScanner<'_> {
        self.0.scanner()
    }

    pub(crate) fn output_type(&self) -> super::MessageScanOutput<'_> {
        super::MessageScanOutput(*self)
    }
}

impl crate::scanner::Parent for MessageScanner<'_> {
    type FieldType = MessageFieldType;
    fn scanner(&self) -> impl crate::scanner::Scanner<FieldType = MessageFieldType> + '_ {
        *self
    }
}

impl crate::scanner::Scanner for MessageScanner<'_> {
    type FieldType = MessageFieldType;
    fn type_name(&self) -> Ident {
        Ident::new(&format!("Scan{}", self.0.name), Span::call_site())
    }

    fn generic_types(&self) -> impl Iterator<Item = FieldGeneric<'_, MessageFieldType>> {
        self.0.fields.iter().map(|f| f.generic())
    }

    fn field_names(&self) -> impl Iterator<Item = Cow<'_, Ident>> {
        self.0
            .fields
            .iter()
            .map(|field| Cow::Borrowed(&field.field_name))
    }
}
