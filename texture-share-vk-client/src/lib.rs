#![feature(unix_socket_ancillary_data)]
//#![allow(dead_code, unused_imports)]

mod bindings;

// cbindgen:ignore
mod vk_client;
pub use vk_client::*;
