use prost_build::Module;
use prost_types::FileDescriptorProto;
use std::io::Result;

pub(crate) mod message;

pub(crate) fn generate_module(module: &Module, fd: FileDescriptorProto) -> Result<String> {
    let FileDescriptorProto {
        name: _,
        package: _,
        dependency: _,
        public_dependency: _,
        weak_dependency: _,
        message_type,
        enum_type: _,
        service: _,
        extension: _,
        options: _,
        source_code_info: _,
        syntax: _,
    } = fd;

    let messages = message_type
        .iter()
        .map(message::generate_message)
        .collect::<Result<Vec<_>>>()?;

    let parts = module
        .parts()
        .map(|m| format!("pub mod {m} {{"))
        .chain(messages)
        .chain(module.parts().map(|_| "}".to_owned()))
        .collect::<Vec<_>>();

    Ok(parts.join("\n"))
}
