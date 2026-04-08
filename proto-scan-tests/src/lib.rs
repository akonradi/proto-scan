pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/testing.rs"));
    include!(concat!(env!("OUT_DIR"), "/empty_message.rs"));
    include!(concat!(env!("OUT_DIR"), "/map.rs"));
    include!(concat!(env!("OUT_DIR"), "/groups.rs"));
    include!(concat!(env!("OUT_DIR"), "/raw_identifiers.rs"));
}

pub mod prost_proto {
    include!(concat!(env!("OUT_DIR"), "/prost/testing.rs"));
    include!(concat!(env!("OUT_DIR"), "/prost/empty_message.rs"));
    include!(concat!(env!("OUT_DIR"), "/prost/map.rs"));
    include!(concat!(env!("OUT_DIR"), "/prost/groups.rs"));
    include!(concat!(env!("OUT_DIR"), "/prost/raw_identifiers.rs"));
}
