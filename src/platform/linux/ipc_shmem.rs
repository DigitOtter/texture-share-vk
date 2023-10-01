use memoffset::offset_of;
use raw_sync::locks::LockGuard;
use raw_sync::locks::LockImpl;
use raw_sync::locks::LockInit;
use raw_sync::locks::ReadLockGuard;
use raw_sync::locks::RwLock;
use raw_sync::Timeout;
use shared_memory::Shmem;
use shared_memory::ShmemConf;
use std::cell::UnsafeCell;
use std::ffi::CStr;
use std::io::{Error, ErrorKind};
use std::mem::size_of;

use crate::platform::img_data::ImgFormat;
use crate::platform::img_data::ImgName;

#[repr(C)]
pub struct ShmemDataInternal {
	name: ImgName,
	handle_id: u32,
	width: u32,
	height: u32,
	format: ImgFormat,
	allocation_size: u32,
}

#[repr(C)]
struct ShmemData {
	lock: RwLock,
	data: UnsafeCell<ShmemDataInternal>,
}

struct IpcShmem {
	shmem: Shmem,
}

impl<'a> IpcShmem {
	pub fn new(
		name: &str,
		img_name: &CStr,
		create: bool,
	) -> Result<IpcShmem, Box<dyn std::error::Error>> {
		let conf = ShmemConf::new().os_id(name).size(size_of::<ShmemData>());
		let shmem = match create {
			true => conf.create().map_err(|e| Box::new(e))?,
			false => conf.open().map_err(|e| Box::new(e))?,
		};

		let raw_shmem_ptr = shmem.as_ptr();
		unsafe {
			let dat_offset = offset_of!(ShmemData, data);
			let raw_data_ptr = raw_shmem_ptr.add(dat_offset);

			let (_lock_data, used_bytes) = RwLock::new(raw_shmem_ptr, raw_data_ptr)?;
			assert!(used_bytes < dat_offset);

			*(raw_data_ptr.cast::<UnsafeCell<ShmemDataInternal>>()) =
				UnsafeCell::new(ShmemDataInternal::new(img_name).map_err(|e| Box::new(e))?);
		};

		Ok(IpcShmem { shmem })
	}

	pub fn acquire_rlock(
		&'a self,
		timeout: Timeout,
	) -> Result<ReadLockGuard<'a>, Box<dyn std::error::Error>> {
		let raw_ptr = self.shmem.as_ptr().cast::<ShmemData>();

		let lock = unsafe { raw_ptr.as_ref().unwrap().lock.try_rlock(timeout)? };
		Ok(lock)
	}

	pub fn acquire_rdata(lock: &'a ReadLockGuard<'a>) -> &'a ShmemDataInternal {
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

	pub fn acquire_lock(
		&'a self,
		timeout: Timeout,
	) -> Result<LockGuard<'a>, Box<dyn std::error::Error>> {
		let raw_ptr = self.shmem.as_ptr().cast::<ShmemData>();

		let lock = unsafe { raw_ptr.as_ref().unwrap().lock.try_lock(timeout)? };
		Ok(lock)
	}

	pub fn acquire_data(lock: &'a LockGuard<'a>) -> &'a mut ShmemDataInternal {
		unsafe {
			lock.cast::<UnsafeCell<ShmemDataInternal>>()
				.as_mut()
				.unwrap()
				.get_mut()
		}
	}
}

impl ShmemDataInternal {
	fn new(img_name: &CStr) -> Result<ShmemDataInternal, Error> {
		if img_name.to_bytes_with_nul().len() > size_of::<ImgName>() {
			Err(Error::new(
				ErrorKind::OutOfMemory,
				format!(
					"Image name '{img_name:?}' too long. Should be less than {:?}bytes",
					size_of::<ImgName>()
				),
			))
		} else {
			let shmem_internal = ShmemDataInternal {
				name: [0; size_of::<ImgName>()],
				handle_id: 0,
				width: 0,
				height: 0,
				format: ImgFormat::Undefined,
				allocation_size: 0,
			};
			Ok(shmem_internal)
		}
	}
}
