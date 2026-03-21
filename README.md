# proto-scan

This is a library for reading data from protocol buffer messages in a streaming context.

The approach to reading protobuf messages enabled by crates like [`prost`] and
the `protoc`-generated bindings is to eagerly deserialize their entire contents
into memory, parsing each field and nested message. This requires memory
allocation and is inefficient if only the contents of a single field need to be
read.

`proto-scan`, by contrast, treats the contents of a message as a sequence of
events which can be read one-by-one from a byte stream. By iterating over these
events and handling them one-by-one, calling code can read a protobuf message
without unnecessary overhead.

## Scanner generation

This library provides two interfaces for message scanning. The high-level
scanning interface relies on compile-time code generation to produce a `Scanner`
builder type for each `message` defined in a .proto file. Methods are defined to
set the behavior for each message field in the original .proto definition.

Producing the scanning code requires either attaching `#[derive]` macros to
`prost-build`-generated message types or using the `proto-scan-build` crate to
generate it from protobuf message descriptors.

## Raw events

In addition to per-message-type code generation, this library also enables
low-level access to protobuf message contents. For lower-level control, the
`proto_scan::wire` and `proto_scan::parse` modules enable typed access to the
contents of a protobuf event stream, where each event corresponds to a tag and its
associated data in the protobuf binary format.

# TODO items remaining
This is still a work in progress. A non-exhaustive list of things remaining:

- Generate enum handling code for type-safe scanning.
- Document `proto-scan-build` and derive processes better (for now see the build script of the -tests crate).
- Make `ScanError::Utf8` variant infallible if none of the scanners check for UTF-8.
- Make the repeated scanners resettable for use in oneof scanners.

[`prost`]: https://docs.rs/prost/latest/prost/
