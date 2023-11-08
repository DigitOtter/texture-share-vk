#![feature(unix_socket_ancillary_data, slice_pattern)]
//#![allow(dead_code, unused_imports)]

mod bindings;

// cbindgen:ignore
mod platform;

// cbindgen:ignore
mod vk_server;
pub use vk_server::*;
