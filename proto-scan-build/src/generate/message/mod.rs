use proc_macro2::{Span, TokenStream};
use prost_types::DescriptorProto;
use quote::{ToTokens, format_ident, quote};
use std::io::Result;
use syn::Ident;

use crate::generate::message::field::{FieldType, MessageScannerField};

pub mod field;

pub struct ProtoMessage {
    message_name: Ident,
    message_fields: Vec<MessageField>,
}

impl ProtoMessage {
    fn scanner(&self) -> ProtoMessageScanner<'_> {
        ProtoMessageScanner(self)
    }

    fn impl_scan_message(&self) -> TokenStream {
        let name = &self.message_name;
        let scanner_name = self.scanner().type_name();
        let no_op = quote!(::proto_scan::scan::field::NoOp);
        let no_ops = std::iter::repeat_n(&no_op, self.message_fields.len());
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

struct ProtoMessageScanner<'m>(&'m ProtoMessage);

impl ProtoMessageScanner<'_> {
    fn type_name(&self) -> Ident {
        Ident::new(&format!("Scan{}", self.0.message_name), Span::call_site())
    }

    fn generic_types(&self) -> impl Iterator<Item = &Ident> + Clone {
        self.0.message_fields.iter().map(|f| &f.generic)
    }

    fn fields(&self) -> impl Iterator<Item = MessageScannerField<'_>> + Clone {
        self.0
            .message_fields
            .iter()
            .enumerate()
            .map(|(index, field)| MessageScannerField {
                parent: self.0,
                index,
                field,
            })
    }

    fn type_definition(&self) -> TokenStream {
        let scanner_name = self.type_name();
        let scan_types = self.generic_types();
        let scan_fields = self.0.message_fields.iter().map(
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
        let scan_fields = self.0.message_fields.iter().map(
            |MessageField {
                 field_name,
                 generic,
                 ..
             }| quote!(#field_name: ::core::option::Option<#generic>),
        );
        quote! {
            #[derive(Copy, Clone, Debug, PartialEq, Hash)]
            pub struct #name <#(#scan_types),*> {
                #(pub #scan_fields ),*
            }

            impl<#(#scan_types),*> ::core::default::Default for #name <#(#scan_types),*> {
                fn default() -> Self {
                    Self {
                        #(#fields: ::core::option::Option::None),*
                    }
                }
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
        let generics_scan_event = generics
            .iter()
            .map(|g| quote!(#g::ScanEvent))
            .collect::<Vec<_>>();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |MessageScannerField { parent: _, index, field: MessageField { field_name, generic: _, field_number, field_type: _ } }| {
                let event_variant_name = format_ident!("Event{index}");
                quote! {
                    #field_number => self.#field_name.#fn_name(value)?.map(#scan_event_name::#event_variant_name)
                }
            })
        };

        let on_scalar_arms = field_arms("on_scalar");
        let on_group_arms = field_arms("on_group");
        let on_length_delimited_arms = field_arms("on_length_delimited");
        quote! {
            impl <#(#generics_on_scan_bounds,)*> ::proto_scan::scan::ScanTypes for #scanner_name<#(#generics,)*> {
                type ScanEvent = Option<#scan_event_name<#(#generics_scan_event),*>>;
                type ScanOutput = #output_name<#(#generics_scan_event),*>;
            }

            impl <#(#generics_on_scan_bounds,)*> ::proto_scan::scan::ScanCallbacks for #scanner_name<#(#generics,)*> {
                fn on_scalar(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: ::proto_scan::wire::ScalarField,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_scalar_arms,)*
                        _ => None,
                    })
                }

                fn on_group(&mut self, field: ::proto_scan::wire::FieldNumber, value: ::proto_scan::wire::GroupOp) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_group_arms,)*
                        _ => None,
                    })
                }

                fn on_length_delimited(
                    &mut self,
                    field: ::proto_scan::wire::FieldNumber,
                    value: impl ::proto_scan::wire::LengthDelimited,
                ) -> Result<Self::ScanEvent, ::proto_scan::scan::StopScan> {
                    Ok(match u32::from(field) {
                        #(#on_length_delimited_arms,)*
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
            .map(|g| quote!(#g: ::proto_scan::scan::field::OnScanField))
            .collect::<Vec<_>>();
        let generics_lifetime_bounds = generics.iter().map(|g| quote!(#g: 'r));
        quote! {
            impl<#(#generics_on_scan_bounds,)*> #scanner_name <#(#generics,)*> {
                pub fn scan<'r>(
                    self,
                    read: impl ::proto_scan::read::Read + 'r,
                ) -> impl ::proto_scan::scan::Scan<
                        ScanEvent = <Self as ::proto_scan::scan::ScanTypes>::ScanEvent,
                        ScanOutput = <Self as ::proto_scan::scan::ScanTypes>::ScanOutput,
                    > + 'r
                where
                    #(#generics_lifetime_bounds,)*
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

    fn output_impl_extend_event(&self) -> TokenStream {
        let output_name = format_ident!("{}Output", self.type_name());
        let scan_event_name = self.scan_event_name();
        let generics = self.generic_types().collect::<Vec<_>>();
        let item_type = quote! {
            ::core::option::Option<
                #scan_event_name<#(#generics),*>
            >
        };
        let field_arms = {
            self.fields().map(move |MessageScannerField { parent: _, index, field: MessageField { field_name, .. } }| {
                let event_variant_name = format_ident!("Event{index}");
                quote! {
                    #scan_event_name::#event_variant_name(t) => state.#field_name = Some(t),                }
            })
        };
        quote! {
            impl <
            #(#generics),*
            > ::core::iter::FromIterator<#item_type> for #output_name<#(#generics),*> {
                fn from_iter<I: ::core::iter::IntoIterator<Item=#item_type>>(items: I) -> Self {
                    let mut state = Self::default();
                    for item in items {
                        let Some(item) = item else {continue};
                        match item {
                            #(#field_arms)*
                        }
                    }
                    state
                }
            }
        }
    }
}

pub struct MessageField {
    field_name: Ident,
    generic: Ident,
    field_number: u32,
    field_type: FieldType,
}

impl TryFrom<&DescriptorProto> for ProtoMessage {
    type Error = std::io::Error;

    fn try_from(message: &DescriptorProto) -> std::result::Result<Self, Self::Error> {
        let DescriptorProto {
            name,
            field,
            extension: _,
            nested_type: _,
            enum_type: _,
            extension_range: _,
            oneof_decl: _,
            options: _,
            reserved_range: _,
            reserved_name: _,
        } = message;
        let name = name
            .as_deref()
            .ok_or_else(|| std::io::Error::other("message has no name"))?;

        let message_fields = field
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let field_name = Ident::new(f.name(), Span::call_site());
                let generic = Ident::new(&format!("T{i}"), Span::call_site());
                Ok(MessageField {
                    field_name,
                    field_number: f
                        .number()
                        .try_into()
                        .map_err(|_| std::io::Error::other("invalid field number"))?,
                    field_type: f.into(),
                    generic,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            message_name: Ident::new(name, Span::call_site()),
            message_fields,
        })
    }
}

impl ToTokens for MessageField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            field_name,
            generic,
            ..
        } = self;
        let field_name = format_ident!("{field_name}");
        tokens.extend(quote! { #field_name: #generic });
    }
}

pub fn generate_message(message: &DescriptorProto) -> Result<String> {
    let message = ProtoMessage::try_from(message)?;
    let scanner = message.scanner();

    let name = &message.message_name;
    let type_defn = quote! {
        pub struct #name;
    };

    let scan_field_impls = scanner.fields().map(|m| m.impl_());

    let scanner_inherent_impls = scanner.inherent_impl();

    Ok([
        type_defn,
        scanner.type_definition(),
        scanner.scan_event_defn(),
        scanner.scan_output_definition(),
        message.impl_scan_message(),
        scanner.scan_callbacks_impl(),
        scanner.output_impl_extend_event(),
    ]
    .into_iter()
    .chain(scan_field_impls)
    .chain([scanner_inherent_impls])
    .collect::<TokenStream>()
    .to_string())
}
