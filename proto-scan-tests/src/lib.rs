pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/testing.rs"));
    include!(concat!(env!("OUT_DIR"), "/empty_message.rs"));
}

pub mod prost_proto {
    include!(concat!(env!("OUT_DIR"), "/prost/testing.rs"));
    include!(concat!(env!("OUT_DIR"), "/prost/empty_message.rs"));
}
