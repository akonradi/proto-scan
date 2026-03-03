use std::borrow::Cow;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{BytesField, Field, MessageField, OneOfField, SingleField};
use crate::oneof::{OneofScannerField, ScannableOneof};
use crate::scanner::Scanner as _;

#[derive(Copy, Clone, Debug)]
pub struct OneofScanner<'a>(&'a ScannableOneof);

impl<'a> OneofScanner<'a> {
    pub(crate) fn new(arg: &'a ScannableOneof) -> Self {
        Self(arg)
    }

    pub fn fields(&self) -> impl Iterator<Item = OneofScannerField<'_>> {
        self.0
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| OneofScannerField {
                inner: crate::scanner::SwapSingleFieldInherentImpl {
                    parent: *self,
                    index: i,
                    field: f,
                },
            })
    }

    fn scan_event_name(&self) -> Ident {
        let scanner_name = self.type_name();
        Ident::new(&format!("{scanner_name}Event"), Span::call_site())
    }

    pub fn scanner_type_definition(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self.fields().map(|f| &f.inner.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.inner.field.field_name;
            let generic = &f.inner.field.generic;
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

    pub fn output_type_definition(&self) -> TokenStream {
        let type_name = self.scan_output_name();
        let generics = self.fields().map(|f| &f.inner.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.inner.field.field_name;
            let generic = &f.inner.field.generic;
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

    pub fn impl_scanner_builder(&self) -> TokenStream {
        let type_name = self.type_name();
        let generics = self
            .fields()
            .map(|f| &f.inner.field.generic)
            .collect::<Vec<_>>();
        let oneof_type_name = &self.0.name;
        let fields = self
            .fields()
            .map(|f| &f.inner.field.field_name)
            .collect::<Vec<_>>();
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

    pub fn event_type_definition(&self) -> TokenStream {
        let type_name = self.scan_event_name();
        let generics = self.fields().map(|f| &f.inner.field.generic);
        let fields = self.fields().map(|f| {
            let name = &f.inner.field.field_name;
            let generic = &f.inner.field.generic;
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

    pub fn impl_scan_callbacks(&self) -> TokenStream {
        let type_name = self.type_name();
        let output_type = self.scan_output_name();
        let field_names = self
            .fields()
            .map(|f| &f.inner.field.field_name)
            .collect::<Vec<_>>();
        let generics = self
            .fields()
            .map(|f| &f.inner.field.generic)
            .collect::<Vec<_>>();
        let generics_with_bounds = self.fields().map(
            |OneofScannerField {
                inner
             }| {
                let generic = &inner.field.generic;
                quote! { #generic: ::proto_scan::scan::field::OnScanField<R> + ::proto_scan::scan::Resettable }
            },
        );
        let last_set_arms = self.fields().map(|OneofScannerField {inner }| {
            let field_name = &inner.field.field_name;
            quote! {
                #output_type::#field_name(()) => #output_type::#field_name(#field_name.into_scan_output())
            }
        });

        let reset_last_set_arms = self.fields().map(|OneofScannerField {inner }| {
            let field_name = &inner.field.field_name;
            quote! {
                #output_type::#field_name(()) => #field_name.reset(),
            }
        });

        let scan_event_name = self.scan_event_name();
        let field_arms = |fn_name: &str| {
            let scan_event_name = &scan_event_name;
            let output_type = &output_type;
            let fn_name = format_ident!("{fn_name}");
            self.fields().map(move |OneofScannerField { inner } | {
                let Field {field_type, field_name, generic: _} = &inner.field;
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
                    let ::core::option::Option::Some(proto_scan_last_set) = proto_scan_last_set else {
                        return
                    };
                    match proto_scan_last_set {
                        #(#reset_last_set_arms)*
                    }
                }
            }

        }
    }

    pub fn scan_output_name(&self) -> Ident {
        format_ident!("{}Output", self.type_name())
    }
}

impl crate::scanner::Parent for OneofScanner<'_> {
    type FieldType = OneOfField;
    fn scanner(&self) -> impl crate::scanner::Scanner<FieldType = Self::FieldType> + '_ {
        *self
    }
}
impl crate::scanner::Scanner for OneofScanner<'_> {
    type FieldType = OneOfField;

    fn type_name(&self) -> Ident {
        format_ident!("Scan{}", self.0.name)
    }

    fn generic_types(&self) -> impl Iterator<Item = crate::field::FieldGeneric<'_, OneOfField>> {
        self.0.fields.iter().map(|f| f.generic())
    }

    fn field_names(&self) -> impl Iterator<Item = Cow<'_, Ident>> {
        self.0
            .fields
            .iter()
            .map(|f| Cow::Borrowed(&f.field_name))
            .chain([Cow::Owned(format_ident!("proto_scan_last_set"))])
    }

    fn output_type(&self) -> impl crate::scanner::ScannerOutput + '_ {
        *self
    }
}

impl crate::scanner::ScannerOutput for OneofScanner<'_> {
    fn type_name(&self) -> Ident {
        self.scan_output_name()
    }
}
