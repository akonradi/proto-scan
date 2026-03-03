use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{Field, MessageFieldType};

pub mod scanner;

/// A type for a protobuf message that can be scanned for.
pub(crate) struct ScannableMessage {
    // The name of the type.
    pub(crate) name: Ident,
    /// The fields in the type.
    pub(crate) fields: Vec<Field<MessageFieldType>>,
}

impl ScannableMessage {
    pub fn scanner(&self) -> scanner::MessageScanner<'_> {
        scanner::MessageScanner::new(self)
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

/// Generator for the output type for a message scanner
#[derive(Copy, Clone)]
pub struct MessageScanOutput<'m>(scanner::MessageScanner<'m>);

impl MessageScanOutput<'_> {
    pub(crate) fn type_name(&self) -> Ident {
        format_ident!("{}Output", self.0.type_name())
    }

    pub fn scan_output_definition(&self) -> TokenStream {
        let name = self.type_name();
        let scan_types = self
            .0
            .generic_types()
            .map(|f| f.ident())
            .collect::<Vec<_>>();
        let scan_fields = self.0.fields().map(|m| &m.field.field_name);
        quote! {
            #[derive(Copy, Clone, Debug, Default, PartialEq, Hash)]
            pub struct #name <#(#scan_types),*> {
                #(pub #scan_fields: #scan_types ),*
            }
        }
    }
}
