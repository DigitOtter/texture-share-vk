#![feature(unix_socket_ancillary_data)]
//#![allow(dead_code, unused_imports)]

mod bindings;

// cbindgen:ignore
pub mod platform;

pub use platform::ipc_shmem::IpcShmem;
pub use platform::ipc_unix_socket::{IpcConnection, IpcSocket};
