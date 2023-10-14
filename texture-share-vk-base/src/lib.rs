#![feature(unix_socket_ancillary_data)]
//#![allow(dead_code, unused_imports)]

pub mod bindings;

// cbindgen:ignore
mod vulkan;
pub use vulkan::*;

pub use cxx;

pub mod ipc {
	pub use texture_share_ipc::*;
}
