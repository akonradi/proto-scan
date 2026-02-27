use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{FieldType, MessageField, MessageScannerField};

pub mod field;

pub struct ScannableMessage {
    pub name: Ident,
    pub fields: Vec<field::MessageField>,
}

impl ScannableMessage {
    pub fn scanner(&self) -> MessageScanner<'_> {
        MessageScanner(self)
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

#[derive(Copy, Clone)]
pub struct MessageScanner<'m>(&'m ScannableMessage);

#[derive(Copy, Clone)]
pub struct MessageScanOutput<'m>(MessageScanner<'m>);

impl MessageScanOutput<'_> {
    fn type_name(&self) -> Ident {
        format_ident!("{}Output", self.0.type_name())
    }

    fn generated_code(&self) -> TokenStream {
        self.scan_output_definition()
    }

    fn scan_output_definition(&self) -> TokenStream {
        let name = self.type_name();
        let scan_types = self.0.generic_types().collect::<Vec<_>>();
        let scan_fields = self.0.fields().map(|m| &m.field.field_name);
        quote! {
            #[derive(Copy, Clone, Debug, Default, PartialEq, Hash)]
            pub struct #name <#(#scan_types),*> {
                #(pub #scan_fields: #scan_types ),*
            }
        }
    }
}

impl MessageScanner<'_> {
    pub fn generated_code(&self) -> TokenStream {
        let scan_field_impls = self.fields().map(|m| m.impl_());

        [
            self.type_definition(),
            self.scan_event_defn(),
            MessageScanOutput(*self).generated_code(),
            self.impl_scanner_builder(),
            self.impl_into_scan(),
            self.0.impl_scan_message(),
            self.scan_callbacks_impl(),
        ]
        .into_iter()
        .chain(scan_field_impls)
        .collect::<TokenStream>()
    }

    fn type_name(&self) -> Ident {
        Ident::new(&format!("Scan{}", self.0.name), Span::call_site())
    }

    fn generic_types(&self) -> impl Iterator<Item = &Ident> + Clone {
        self.0.fields.iter().map(|f| f.generic())
    }

    fn fields(&self) -> impl Iterator<Item = MessageScannerField<'_>> + Clone {
        self.0
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| MessageScannerField {
                parent: *self,
                index,
                field,
            })
    }

    fn type_definition(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let scan_types = self.generic_types();
        let scan_fields = self.0.fields.iter().map(
            |MessageField {
                 field_name,
                 generic,
                 ..
             }| quote!(#field_name: #generic),
        );
        quote! {
            #[derive(Default)]
            pub struct #scanner_name <#(#scan_types),*> {
                #(#scan_fields, )*
            }
        }
    }

    fn field_names(&self) -> impl Iterator<Item = &Ident> + Clone {
        self.fields().map(|m| &m.field.field_name)
    }

    fn scan_event_defn(&self) -> TokenStream {
        let scan_event_name = self.scan_event_name();
        let generics = self.generic_types();
        let variants = generics.clone().enumerate().map(|(i, t)| {
            let name = format_ident!("Event{i}");
            quote!(#name(#t))
        });
        quote! {
            pub enum #scan_event_name<#(#generics,)*> {
                #(#variants),*
            }
        }
    }

    fn impl_scanner_builder(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let message_name = &self.0.name;
        quote! {
            impl<#(#generics: ::proto_scan::scan::IntoScanner ),*> ::proto_scan::scan::ScannerBuilder for #type_name < #(#generics),* > {
                type Message = #message_name;
            }
        }
    }

    fn impl_into_scan(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let field_names = self.field_names().collect::<Vec<_>>();
        quote! {
            impl<#(#generics: ::proto_scan::scan::IntoScanner),*> ::proto_scan::scan::IntoScanner for #type_name < #(#generics),* > {
                type Scanner<R: ::proto_scan::read::ReadTypes> = #type_name < #(<#generics as ::proto_scan::scan::IntoScanner>::Scanner<R> ),* >;
                fn into_scanner<R: ::proto_scan::read::ReadTypes>(self) -> Self::Scanner<R> {
                    let Self { #(#field_names),* } = self;
                    Self::Scanner {
                        #(#field_names: #field_names.into_scanner()),*
                    }

                }
            }
        }
    }

    fn scan_callbacks_impl(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let output_name = MessageScanOutput(*self).type_name();
        let scan_event_name = self.scan_event_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let generics_on_scan_bounds = generics
            .iter()
            .map(|g| quote!(#g: ::proto_scan::scan::field::OnScanField<R>))
            .collect::<Vec<_>>();
        let field_names = self.field_names().collect::<Vec<_>>();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |MessageScannerField { parent: _, index, field: MessageField { field_name, generic: _, field_type } }| {
                let event_variant_name = format_ident!("Event{index}");
                match field_type {
                    FieldType::Single { ty: _, number } | FieldType::Repeated { ty: _, number } | FieldType::Message { number, type_name: _ } | FieldType::Bytes { utf8: _, number } => quote! {
                        #number => self.#field_name.#fn_name(value)?.map(#scan_event_name::#event_variant_name),
                    },
                    FieldType::Unsupported => TokenStream::new()
                }
            })
        };

        let on_numeric_arms = field_arms("on_numeric");
        let on_group_arms = field_arms("on_group");
        let on_length_delimited_arms = field_arms("on_length_delimited");
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

            impl <#(#generics_on_scan_bounds),* , R: ::proto_scan::read::ReadTypes> ::proto_scan::scan::ScanCallbacks<R> for #scanner_name<#(#generics,)*> {
                type ScanEvent = Option<#scan_event_name<#(#generics :: ScanEvent),*>>;
                fn on_numeric(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: ::proto_scan::wire::NumericField,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_numeric_arms)*
                        _ => None,
                    })
                }

                fn on_group(&mut self, field: ::proto_scan::wire::FieldNumber, value: ::proto_scan::wire::GroupOp) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_group_arms)*
                        _ => None,
                    })
                }

                fn on_length_delimited(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: impl ::proto_scan::wire::LengthDelimited<ReadTypes=R>,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
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
}
