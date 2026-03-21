use proc_macro2::TokenStream;
use quote::quote;

use crate::field::{Field, MessageFieldType};
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

        let into_scanner_type = field_type.as_into_scanner_type();
        let custom_fn = SwapSingleFieldFn {
            fn_verb: "",
            docs: &[
                &format!("Sets the field scanner for message field `{field_name}`."),
                "",
                &format!(
                    "This allows the caller to specify the behavior on
                    encountering the field `{field_name}` defined in the source
                    message. The output of the provided field scanner will be
                    included in the overall scan output as
                    [`{output_type}::{field_name}`]."
                ),
            ],
            generics: &[
                quote!('t),
                quote!(S: ::proto_scan::scan::IntoScanner<#into_scanner_type> + 't),
            ],
            args: &[quote!(scanner: S)],
            output_type: quote!(S),
            construct_field: quote!(scanner),
        };

        inner.generate_fns([custom_fn])
    }
}
