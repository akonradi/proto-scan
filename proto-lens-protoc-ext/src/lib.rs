use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::Result;
use std::path::Path;

use log::trace;
use proc_macro2::TokenStream;
use prost::Message;
use prost_build::Module;
use prost_types::{DescriptorProto, FileDescriptorProto, FileDescriptorSet, compiler::*};
use quote::{ToTokens, format_ident, quote};

pub(crate) mod generate;

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
        let target = std::env::var("OUT_DIR").map_err(|e| std::io::Error::other(e))?;
        let target = Path::new(&target);

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

        let file_names = requests
            .iter()
            .map(|req| (req.0.clone(), req.0.to_file_name_or(&"_")))
            .collect::<HashMap<Module, String>>();

        let modules = generate(requests)?;
        for (module, content) in &modules {
            let file_name = file_names
                .get(module)
                .expect("every module should have a filename");
            let output_path = target.join(file_name);

            println!("writing to {output_path:?}: {}", content.len());
            write_file_if_changed(&output_path, content.as_bytes())?;

            if let Ok(p) = std::env::var("RUSTFMT")
                && !p.is_empty()
            {}
        }

        Ok(())
    }
}

fn generate(requests: Vec<(Module, FileDescriptorProto)>) -> Result<HashMap<Module, String>> {
    let mut output = HashMap::default();
    for (module, fd) in requests {
        let name = &fd.name;
        println!("generating for {name:?} as {module:?}");
        let contents = generate::generate_module(&module, fd)?;

        output.insert(module, contents);
    }

    Ok(output)
}

/// Write a slice as the entire contents of a file.
///
/// This function will create a file if it does not exist,
/// and will entirely replace its contents if it does. When
/// the contents is already correct, it doesn't touch to the file.
fn write_file_if_changed(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let previous_content = fs::read(path);

    if previous_content
        .map(|previous_content| previous_content == content)
        .unwrap_or(false)
    {
        trace!("unchanged: {}", path.display());
        Ok(())
    } else {
        trace!("writing: {}", path.display());
        fs::write(path, content)
    }
}
