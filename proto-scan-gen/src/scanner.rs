use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::field::{Field, FieldGeneric};

pub(crate) struct SwapSingleFieldInherentImpl<'m, P, F> {
    pub(crate) parent: P,
    pub(crate) index: usize,
    pub(crate) field: &'m Field<F>,
}

pub(crate) trait Parent {
    type FieldType;
    fn scanner(&self) -> impl Scanner<FieldType = Self::FieldType> + '_;
}

pub(crate) trait Scanner {
    type FieldType;
    fn type_name(&self) -> Ident;
    fn generic_types(&self) -> impl Iterator<Item = FieldGeneric<'_, Self::FieldType>>;
    fn field_names(&self) -> impl Iterator<Item = Cow<'_, Ident>>;
}

pub(crate) trait ScannerOutput {
    fn type_name(&self) -> Ident;
}

#[derive(Default)]
pub(crate) struct SwapSingleFieldFn<'a> {
    pub(crate) fn_verb: &'static str,
    pub(crate) docs: &'a [&'a str],
    pub(crate) generics: &'a [TokenStream],
    pub(crate) args: &'a [TokenStream],
    pub(crate) output_type: TokenStream,
    pub(crate) construct_field: TokenStream,
}

impl<'m, P: Parent, F> SwapSingleFieldInherentImpl<'m, P, F> {
    pub(crate) fn generate_fns<'a>(
        &self,
        impl_fns: impl IntoIterator<Item = SwapSingleFieldFn<'a>>,
    ) -> TokenStream {
        let Self {
            parent,
            index,
            field,
        } = self;
        let scanner = parent.scanner();
        let scanner_name = scanner.type_name();
        let generic_types = scanner
            .generic_types()
            .map(FieldGeneric::ident)
            .collect::<Vec<_>>();
        let (before_no_op, tail) = generic_types.split_at(*index);
        let (_, after_no_op) = tail.split_first().unwrap();
        let scanner_fields = scanner.field_names().collect::<Vec<_>>();
        let field_name = &field.field_name;

        let impl_fns = impl_fns.into_iter().map(
            |SwapSingleFieldFn {
                 fn_verb,
                 docs,
                 generics,
                 args,
                 output_type,
                 construct_field,
             }| {
                let sep = if fn_verb.is_empty() { "" } else { "_" };
                let fn_name = format_ident!("{fn_verb}{sep}{field_name}");
                quote! {
                    #( #[doc = #docs] )*
                    #[allow(clippy::type_complexity)]
                    pub fn #fn_name <#(#generics),*>(
                        self,
                        #(#args),*
                    ) -> #scanner_name<
                            #(#before_no_op,)*
                            #output_type,
                            #(#after_no_op,)*
                    > {
                        let Self { #(#scanner_fields,)* } = self;
                        let _ = #field_name;
                        let #field_name = #construct_field;
                        #scanner_name { #(#scanner_fields,)* }
                    }
                }
            },
        );

        quote! {
            impl< #(#before_no_op,)* #(#after_no_op),* > #scanner_name< #(#before_no_op,)* ::proto_scan::scan::field::NoOp, #(#after_no_op),*> {
                #(#impl_fns)*
            }
        }
    }
}
