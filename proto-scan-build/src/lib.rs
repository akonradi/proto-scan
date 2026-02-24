use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Result;
use std::path::Path;

use prost_build::Module;
use prost_types::FileDescriptorProto;

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
        let cargo_cmd = std::env::var_os("CARGO");
        for (module, content) in &modules {
            let file_name = file_names
                .get(module)
                .expect("every module should have a filename");
            let output_path = target.join(file_name);

            println!("writing to {output_path:?}: {}", content.len());
            fs::write(&output_path, content.as_bytes())?;

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

fn generate(requests: Vec<(Module, FileDescriptorProto)>) -> Result<HashMap<Module, String>> {
    let mut output = HashMap::default();
    for (module, fd) in requests {
        let name = &fd.name;
        println!("generating for {name:?} as {module:?}");
        let contents = generate::generate_module(fd)?;

        output.insert(module, contents);
    }

    Ok(output)
}
