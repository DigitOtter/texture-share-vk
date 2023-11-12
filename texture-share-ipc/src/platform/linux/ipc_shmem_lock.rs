use memoffset::offset_of;
use raw_sync::{
	locks::{LockImpl, LockInit, RwLock},
	Timeout,
};
use shared_memory::Shmem;
use std::cell::UnsafeCell;

use crate::{
	platform::{ipc_shmem::ShmemData, ShmemDataInternal},
	IpcShmem,
};

pub type RwLockInternalData = libc::pthread_rwlock_t;

pub type ReadLockGuard<'a> = raw_sync::locks::ReadLockGuard<'a>;
pub type LockGuard<'a> = raw_sync::locks::LockGuard<'a>;

impl IpcShmem {
	pub(crate) fn init_rw_lock(
		shmem: &Shmem,
		from_existing: bool,
	) -> Result<Box<dyn LockImpl>, Box<dyn std::error::Error>> {
		let raw_rwlock_ptr = unsafe { shmem.as_ptr().add(offset_of!(ShmemData, rwlock_data)) };
		let raw_data_ptr = unsafe { shmem.as_ptr().add(offset_of!(ShmemData, data)) };

		let res = unsafe {
			if !from_existing {
				RwLock::new(raw_rwlock_ptr, raw_data_ptr)
			} else {
				RwLock::from_existing(raw_rwlock_ptr, raw_data_ptr)
			}
		}?;
		assert!(res.1 <= offset_of!(ShmemData, data) - offset_of!(ShmemData, rwlock_data));

		Ok(res.0)
	}

	pub fn acquire_rlock<'a>(
		&'a self,
		timeout: Timeout,
	) -> Result<ReadLockGuard<'a>, Box<dyn std::error::Error>> {
		self.lock.try_rlock(timeout)
	}

	pub fn acquire_rdata<'a>(lock: &ReadLockGuard<'a>) -> &'a ShmemDataInternal {
		unsafe {
			lock.cast::<UnsafeCell<ShmemDataInternal>>()
				.as_ref()
				.unwrap()
				.get()
				.as_ref()
				.unwrap()
		}
	}

	// fn read_lock(
	// 	&self,
	// 	timeout: Timeout,
	// ) -> Result<(&ShmemDataInternal, ReadLockGuard), Box<dyn std::error::Error>> {
	// 	let raw_ptr = self.shmem.as_ptr().cast::<ShmemData>();

	// 	let lock = unsafe { raw_ptr.as_ref().unwrap().lock.try_rlock(timeout)? };
	// 	let data = unsafe { raw_ptr.as_ref().unwrap().data.get().as_ref().unwrap() };

	// 	Ok((data, lock))
	// }

	pub fn acquire_lock<'a>(
		&'a self,
		timeout: Timeout,
	) -> Result<LockGuard<'a>, Box<dyn std::error::Error>> {
		self.lock.try_lock(timeout)
	}

	pub fn acquire_data<'a>(lock: &'a LockGuard<'a>) -> &'a mut ShmemDataInternal {
		unsafe {
			lock.cast::<UnsafeCell<ShmemDataInternal>>()
				.as_mut()
				.unwrap()
				.get_mut()
		}
	}
}
