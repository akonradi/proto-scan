#![doc(hidden)]

use prost_build::Module;
use prost_types::FileDescriptorSet;
use std::ffi::OsStr;
use std::io::Error;
use std::path::{Path, PathBuf};

use super::{Result, generate_prost};

#[derive(Debug, Default, derive_more::From)]
pub struct Config(prost_build::Config);

impl Config {
    /// Compile the set of input .proto files with the given include directories.
    pub fn compile_protos(
        &mut self,
        inputs: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> Result<()> {
        let fds = self.0.load_fds(inputs, includes)?;
        self.compile_fds(fds)
    }

    /// Compile the protos in a [`FileDescriptorSet`].
    pub fn compile_fds(&mut self, fds: FileDescriptorSet) -> Result<()> {
        let mut target_is_env = false;
        let target: PathBuf = std::env::var_os("OUT_DIR")
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
            .collect();

        let prost_gen = self.0.generate(requests)?;

        let modules = generate_prost(prost_gen)?;
        let cargo_cmd = std::env::var_os("CARGO");
        for (module, content) in &modules {
            let file_name = module.to_file_name_or("_");
            let output_path = target.join(file_name);

            std::fs::write(&output_path, content.to_string().as_bytes())?;

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

/// Convenience methods for configuring compilation.
impl Config {
    /// See [`prost_build::Config::out_dir`].
    pub fn out_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.0.out_dir(path);
        self
    }

    /// See [`prost_build::Config::protoc_arg`].
    pub fn protoc_arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.0.protoc_arg(arg);
        self
    }

    /// See [`prost_build::Config::compile_well_known_types`].
    pub fn compile_well_known_types(&mut self) -> &mut Self {
        self.0.compile_well_known_types();
        self
    }
}
