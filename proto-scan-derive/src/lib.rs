#[proc_macro_derive(ScanMessage)]
pub fn scan_message_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    proto_scan_gen::prost::derive_impl(input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
