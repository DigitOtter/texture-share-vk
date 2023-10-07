#![feature(unix_socket_ancillary_data)]
//#![allow(dead_code, unused_imports)]

mod bindings;

// cbindgen:ignore
pub mod platform {
    pub mod img_data;
    pub mod ipc_commands;

    #[cfg(target_os = "linux")]
    mod linux;
    #[cfg(target_os = "linux")]
    pub(crate) use linux::*;

    pub use ipc_shmem::ShmemDataInternal;
    pub use ipc_shmem::{LockGuard, ReadLockGuard, Timeout};
}

pub use platform::ipc_shmem::IpcShmem;
pub use platform::ipc_unix_socket::{IpcConnection, IpcSocket};
