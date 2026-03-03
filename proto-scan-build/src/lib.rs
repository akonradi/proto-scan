use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::{env, fs};

use itertools::Itertools;
use proc_macro2::TokenStream;
use prost_build::Module;
use quote::{ToTokens, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Token, parse_quote};

pub trait CompileScan {
    fn compile_scan(
        &mut self,
        inputs: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> Result<()>;
}

impl CompileScan for prost_build::Config {
    fn compile_scan(
        &mut self,
        inputs: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> Result<()> {
        let fds = self.load_fds(inputs, includes)?;

        let mut target_is_env = false;
        let target: PathBuf = env::var_os("OUT_DIR")
            .ok_or_else(|| Error::other("OUT_DIR environment variable is not set"))
            .map(|val| {
                target_is_env = true;
                Into::into(val)
            })?;

        let requests = fds
            .file
            .into_iter()
            .map(|descriptor| {
                (
                    Module::from_protobuf_package_name(descriptor.package()),
                    descriptor,
                )
            })
            .collect::<Vec<_>>();

        let prost_gen = self.generate(requests)?;

        let modules = generate_prost(prost_gen)?;
        let cargo_cmd = std::env::var_os("CARGO");
        for (module, content) in &modules {
            let file_name = module.to_file_name_or("_");
            let output_path = target.join(file_name);

            fs::write(&output_path, content.to_string().as_bytes())?;

            if let Some(cargo_cmd) = &cargo_cmd {
                let cmd = std::process::Command::new(cargo_cmd)
                    .args([OsStr::new("fmt"), OsStr::new("--"), output_path.as_os_str()])
                    .status()?;
                if !cmd.success() {
                    eprintln!("cargo fmt failed");
                }
            }
        }

        Ok(())
    }
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
