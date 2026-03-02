// TODO reenable this
// pub mod proto {
//     include!(concat!(env!("OUT_DIR"), "/testing.rs"));
// }

pub mod prost_proto {
    include!(concat!(env!("OUT_DIR"), "/prost/testing.rs"));
}
