use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::field::{Field, OneOfField};
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

    pub fn impl_scannable_oneof(&self) -> TokenStream {
        let type_name = &self.name;
        let field_number = self.scanner().field_number_type_name();
        quote ![
            impl ::proto_scan::scan::ScannableOneOf for #type_name {
                type FieldNumber = #field_number;
            }
        ]
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
        let field_variant = &inner.field.variant_name;

        let into_scanner_type = field_type.as_into_scanner_type();
        let custom_fn = SwapSingleFieldFn {
            docs: &[
                &format!("Sets the field scanner for oneof field `{field_name}`."),
                "",
                &format!(
                    "This allows the caller to specify the behavior on
                    encountering the field `{field_name}` defined in the source
                    oneof. The output of the provided field scanner will be
                    included in the overall scan output as
                    [`{output_type}::{field_variant}`]."
                ),
            ],
            generics: &[
                quote!('t),
                quote!(S: ::proto_scan::scan::IntoScanner<#into_scanner_type, Scanner<::proto_scan::read::BoundsOnlyReadTypes>: ::proto_scan::scan::ResettableScanner> + 't),
            ],
            args: &[quote!(scanner: S)],
            output_type: quote!(S),
            construct_field: quote!(scanner),
        };

        inner.generate_fns([custom_fn])
    }
}
