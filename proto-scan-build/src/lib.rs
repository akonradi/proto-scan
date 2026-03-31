use std::collections::HashMap;
use std::io::{Error, Result};
use std::path::Path;

use itertools::Itertools;
use proc_macro2::TokenStream;
use prost_build::Module;
pub use prost_types::FileDescriptorSet;
use quote::{ToTokens, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Token, parse_quote};

mod config;
pub use config::Config;

pub fn compile_protos(protos: &[impl AsRef<Path>], includes: &[impl AsRef<Path>]) -> Result<()> {
    Config::default().compile_protos(protos, includes)
}

pub fn compile_fds(fds: FileDescriptorSet) -> Result<()> {
    Config::default().compile_fds(fds)
}

fn generate_prost(prost_gen: HashMap<Module, String>) -> Result<HashMap<Module, TokenStream>> {
    let mut output = HashMap::default();
    for (module, prost_gen) in prost_gen {
        println!("generating for {module:?}");

        let parsed: syn::ItemMod = syn::parse_str(&format!("mod m {{ {prost_gen} }}")).unwrap();

        let contents = visit_mod(&parsed.content.unwrap().1).map_err(Error::other)?;

        output.insert(module, contents);
    }

    Ok(output)
}

fn visit_mod(mod_items: &[syn::Item]) -> syn::Result<TokenStream> {
    let prost_message: syn::Path = parse_quote!(::prost::Message);
    let prost_oneof: syn::Path = parse_quote!(::prost::Oneof);

    mod_items
        .iter()
        .map(|item| {
            let item: TokenStream = match item {
                syn::Item::Enum(item_enum) => {
                    let Some(derive) = item_enum
                        .attrs
                        .iter()
                        .find(|a| a.meta.path().is_ident("derive"))
                    else {
                        return Ok(None);
                    };
                    let parsed = Punctuated::<syn::Path, Token![,]>::parse_separated_nonempty
                        .parse2(derive.meta.require_list()?.tokens.clone())?;
                    if !parsed.iter().any(|p| *p == prost_oneof) {
                        return Ok(None);
                    }
                    let name = &item_enum.ident;
                    quote! {pub enum #name {

                    } }
                    .into_iter()
                    .chain(proto_scan_gen::prost::derive_impl(
                        parse_quote!(#item_enum),
                    )?)
                    .collect()
                }
                syn::Item::Struct(item_struct) => {
                    let Some(derive) = item_struct
                        .attrs
                        .iter()
                        .find(|a| a.meta.path().is_ident("derive"))
                    else {
                        return Ok(None);
                    };
                    let parsed = Punctuated::<syn::Path, Token![,]>::parse_separated_nonempty
                        .parse2(derive.meta.require_list()?.tokens.clone())?;
                    if !parsed.iter().any(|p| *p == prost_message) {
                        return Ok(None);
                    }

                    let name = &item_struct.ident;
                    quote! {pub struct #name (::core::convert::Infallible); }
                        .into_iter()
                        .chain(proto_scan_gen::prost::derive_impl(
                            parse_quote!(#item_struct),
                        )?)
                        .collect()
                }
                syn::Item::Mod(module) => {
                    let module_tokens = module
                        .content
                        .as_ref()
                        .map(|(_, t)| visit_mod(t))
                        .transpose()?;
                    let items: syn::ItemMod = syn::parse_quote! { mod m { #module_tokens } };
                    let module = syn::ItemMod {
                        content: items.content,
                        ..module.clone()
                    };

                    module.into_token_stream()
                }
                _ => return Ok(None),
            };
            Ok(Some(item))
        })
        .flatten_ok()
        .try_collect()
}
