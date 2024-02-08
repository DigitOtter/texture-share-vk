#![feature(entry_insert)]

mod bindings;

// cbindgen:ignore
mod platform;

// cbindgen:ignore
mod vk_server;
pub use vk_server::*;
