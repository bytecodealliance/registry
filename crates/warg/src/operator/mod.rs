pub mod model;
pub mod validate;

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/warg.operator.rs"));
}
