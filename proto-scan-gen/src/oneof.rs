use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{self, BytesField, Field, MessageField, OneOfField, SingleField};

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

pub struct OneofScanner<'a>(&'a ScannableOneof);

pub struct OneofScannerField<'a>(&'a Field<OneOfField>);

impl<'a> OneofScanner<'a> {
    fn type_name(&self) -> Ident {
        format_ident!("Scan{}", self.0.name)
    }

    pub fn fields(&self) -> impl Iterator<Item = OneofScannerField<'_>> {
        self.0.fields.iter().map(OneofScannerField)
    }

    fn scan_event_name(&self) -> Ident {
        let scanner_name = self.type_name();
        Ident::new(&format!("{scanner_name}Event"), Span::call_site())
    }

    pub fn generated_code(&self) -> TokenStream {
        [
            self.scanner_type_definition(),
            self.output_type_definition(),
            self.event_type_definition(),
            self.impl_scan_callbacks(),
        ]
        .into_iter()
        .collect()
    }

    fn scanner_type_definition(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.fields().map(|f| &f.0.generic);
        let fields = self.fields().map(|f| {
            let name = &f.0.field_name;
            let generic = &f.0.generic;
            quote! {
                #name: #generic,
            }
        });
        let last_set_type = self.scan_output_name();
        quote! {
            pub struct #type_name<#(#generics),*> {
                #(#fields)*
                proto_scan_last_set: ::core::option::Option<#last_set_type>,
            }
        }
    }

    fn output_type_definition(&self) -> TokenStream {
        let type_name = self.scan_output_name();
        let generics = self.fields().map(|f| &f.0.generic);
        let fields = self.fields().map(|f| {
            let name = &f.0.field_name;
            let generic = &f.0.generic;
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

    fn event_type_definition(&self) -> TokenStream {
        let type_name = self.scan_event_name();
        let generics = self.fields().map(|f| &f.0.generic);
        let fields = self.fields().map(|f| {
            let name = &f.0.field_name;
            let generic = &f.0.generic;
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
        let field_names = fields.iter().map(|f| &f.0.field_name).collect::<Vec<_>>();
        let generics = fields.iter().map(|f| &f.0.generic).collect::<Vec<_>>();
        let generics_with_bounds = fields.iter().map(
            |OneofScannerField(Field {
                 field_name,
                 generic,
                 field_type,
             })| {
                quote! { #generic: ::proto_scan::scan::field::OnScanField<R> }
            },
        );
        let last_set_arms = fields.iter().map(|OneofScannerField(Field { field_name, generic, field_type })| {
            quote! {
                #output_type::#field_name(()) => #output_type::#field_name(#field_name.into_scan_output())
            }
        });

        let scan_event_name = self.scan_event_name();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let output_type = &output_type;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |OneofScannerField(Field { field_name, generic: _, field_type }) | {
                let event_variant_name = field_name;
                match field_type {
                    OneOfField::Single(SingleField { ty: _, number })
                    | OneOfField::Message(MessageField{ number, type_name: _ })
                    | OneOfField::Bytes(BytesField{ utf8: _, number }) => quote! {
                        #number => {
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

        }
    }

    fn scan_output_name(&self) -> Ident {
        format_ident!("{}Output", self.type_name())
    }
}
