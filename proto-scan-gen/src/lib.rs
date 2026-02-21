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
                type Scanner = #scanner_name <#(#no_ops),*>;

                fn scanner() -> Self::Scanner {
                    ::core::default::Default::default()
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct MessageScanner<'m>(&'m ScannableMessage);

impl MessageScanner<'_> {
    pub fn generated_code(&self) -> TokenStream {
        let scan_field_impls = self.fields().map(|m| m.impl_());

        let scanner_inherent_impls = self.inherent_impl();

        [
            self.type_definition(),
            self.scan_event_defn(),
            self.scan_output_definition(),
            self.0.impl_scan_message(),
            self.scan_callbacks_impl(),
        ]
        .into_iter()
        .chain(scan_field_impls)
        .chain([scanner_inherent_impls])
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

    fn scan_output_definition(&self) -> TokenStream {
        let name = format_ident!("{}Output", self.type_name());
        let scan_types = self.generic_types().collect::<Vec<_>>();
        let fields = self.field_names();
        let scan_fields = self.0.fields.iter().map(
            |MessageField {
                 field_name,
                 generic,
                 ..
             }| quote!(#field_name: #generic),
        );
        quote! {
            #[derive(Copy, Clone, Debug, Default, PartialEq, Hash)]
            pub struct #name <#(#scan_types),*> {
                #(pub #scan_fields ),*
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

    fn scan_callbacks_impl(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let output_name = format_ident!("{scanner_name}Output");
        let scan_event_name = self.scan_event_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let generics_on_scan_bounds = generics
            .iter()
            .map(|g| quote!(#g: ::proto_scan::scan::field::OnScanField))
            .collect::<Vec<_>>();
        let field_names = self.field_names().collect::<Vec<_>>();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |MessageScannerField { parent: _, index, field: MessageField { field_name, generic: _, field_type } }| {
                let event_variant_name = format_ident!("Event{index}");
                match field_type {
                    FieldType::Single { ty: _, number } | FieldType::Repeated { ty: _, number } => quote! {
                        #number => self.#field_name.#fn_name(value)?.map(#scan_event_name::#event_variant_name),
                    },
                    FieldType::Unsupported => TokenStream::new()
                }
            })
        };

        let on_scalar_arms = field_arms("on_scalar");
        let on_group_arms = field_arms("on_group");
        let on_length_delimited_arms = field_arms("on_length_delimited");
        quote! {
            impl <#(#generics_on_scan_bounds,)*> ::proto_scan::scan::ScanTypes for #scanner_name<#(#generics,)*> {
                type ScanEvent = Option<#scan_event_name<#(#generics :: ScanEvent),*>>;
                type ScanOutput = #output_name<#(#generics::ScanOutput),*>;
            }

            impl <#(#generics_on_scan_bounds,)*> ::core::convert::From<#scanner_name<#(#generics,)*>> for #output_name<#(#generics::ScanOutput),*> {
                fn from(scanner: #scanner_name<#(#generics),*>) -> Self {
                    let #scanner_name { #(#field_names),* } = scanner;
                    Self {
                        #(#field_names: #field_names.into_output(),)*
                    }
                }
            }

            impl <#(#generics_on_scan_bounds,)*> ::proto_scan::scan::ScanCallbacks for #scanner_name<#(#generics,)*> {
                fn on_scalar(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: ::proto_scan::wire::ScalarField,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_scalar_arms)*
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
                    value: impl ::proto_scan::wire::LengthDelimited,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_length_delimited_arms)*
                        _ => None,
                    })
                }
            }
        }
    }

    fn inherent_impl(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let generics_on_scan_bounds = generics
            .iter()
            .map(|g| quote!(#g: ::proto_scan::scan::field::OnScanField + 'r))
            .collect::<Vec<_>>();
        quote! {
            impl<'r, #(#generics_on_scan_bounds,)*> ::proto_scan::scan::Scanner<'r> for #scanner_name <#(#generics,)*> {
                fn scan(
                    self,
                    read: impl ::proto_scan::read::Read + 'r,
                ) -> impl ::proto_scan::scan::Scan<
                        ScanEvent = <Self as ::proto_scan::scan::ScanTypes>::ScanEvent,
                        ScanOutput = <Self as ::proto_scan::scan::ScanTypes>::ScanOutput,
                    > + 'r
                {
                    ::proto_scan::scan::ScanWith::new(::proto_scan::wire::parse(read), self)
                }
            }
        }
    }

    fn scan_event_name(&self) -> Ident {
        let scanner_name = self.type_name();
        Ident::new(&format!("{scanner_name}Event"), Span::call_site())
    }
}
