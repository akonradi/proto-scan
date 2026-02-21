pub mod read;
pub mod scan;
pub mod wire;

mod decode_error;
use decode_error::DecodeError;

#[cfg(feature = "derive")]
pub use proto_scan_derive::ScanMessage;