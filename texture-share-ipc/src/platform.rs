pub mod daemon_launch;
pub mod img_data;
pub mod ipc_commands;
pub mod ipc_shmem;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::*;

pub use ipc_shmem_lock::{LockGuard, ReadLockGuard, RwLockInternalData};

pub use ipc_shmem::ShmemDataInternal;
pub use ipc_shmem::Timeout;
