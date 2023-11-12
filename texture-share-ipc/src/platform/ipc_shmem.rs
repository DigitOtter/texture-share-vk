use memoffset::offset_of;
use raw_sync::locks::LockImpl;
pub use raw_sync::Timeout;
use shared_memory::Shmem;
use shared_memory::ShmemConf;
use shared_memory::ShmemError;
use std::cell::UnsafeCell;

use std::io::{Error, ErrorKind};
use std::mem::size_of;

use crate::platform::img_data::ImgData;
use crate::platform::img_data::ImgFormat;
use crate::platform::img_data::ImgName;
use crate::platform::RwLockInternalData;

#[repr(C)]
#[derive(Clone)]
pub struct ShmemDataInternal {
	pub name: ImgName,
	pub handle_id: u32,
	pub width: u32,
	pub height: u32,
	pub format: ImgFormat,
	pub allocation_size: u64,
}

#[repr(C)]
pub(super) struct ShmemData {
	pub(super) rwlock_data: RwLockInternalData,
	pub(super) data: UnsafeCell<ShmemDataInternal>,
}

pub struct IpcShmem {
	pub(super) lock: Box<dyn LockImpl>,
	shmem: Shmem,
}

impl Default for ShmemDataInternal {
	fn default() -> Self {
		ShmemDataInternal {
			name: [0 as u8; size_of::<ImgName>()],
			handle_id: 0,
			width: 0,
			height: 0,
			format: ImgFormat::default(),
			allocation_size: 0,
		}
	}
}

impl<'a> IpcShmem {
	pub fn new(
		name: &str,
		img_name: &str,
		create: bool,
	) -> Result<IpcShmem, Box<dyn std::error::Error>> {
		let conf = ShmemConf::new().os_id(name).size(size_of::<ShmemData>());
		let shmem = if create {
			match conf.clone().create() {
				Err(e) => match e {
					ShmemError::MappingIdExists => {
						IpcShmem::delete_shmem(name);
						conf.create()?
					}
					_ => Err(e)?,
				},
				Ok(s) => s,
			}
		} else {
			conf.open().map_err(|e| Box::new(e))?
		};

		// match create {
		//     true => conf.create().map_err(|e| Box::new(e))?,
		//     false => conf.open().map_err(|e| Box::new(e))?,
		// };

		let lock = IpcShmem::init_rw_lock(&shmem, !create)?;

		if create {
			let _rw_lock = lock.lock()?;
			unsafe {
				let raw_data_ptr = shmem.as_ptr().add(offset_of!(ShmemData, data));
				*(raw_data_ptr.cast::<UnsafeCell<ShmemDataInternal>>()) = UnsafeCell::new(
					ShmemDataInternal::new_empty(img_name).map_err(|e| Box::new(e))?,
				);
			}
		}

		Ok(IpcShmem { lock, shmem })
	}

	// Get Id without acquiring lock. Should be sufficient for checking if surface image has changed
	pub fn get_id_unchecked(&self) -> u32 {
		unsafe {
			(self.shmem.as_ptr().add(offset_of!(ShmemData, data)) as *const ShmemDataInternal)
				.as_ref()
				.unwrap()
		}
		.handle_id
	}

	pub fn get_name(&self) -> &str {
		self.shmem.get_os_id()
	}

	fn delete_shmem(shmem_name: &str) {
		let conf = ShmemConf::new().os_id(shmem_name);

		let shmem = conf.open();
		if let Err(_) = shmem {
			return;
		}

		// Set as owner to delete on drop
		let mut shmem = shmem.unwrap();
		shmem.set_owner(true);
	}
}

impl ShmemDataInternal {
	pub(crate) fn new(
		name: ImgName,
		handle_id: u32,
		width: u32,
		height: u32,
		format: ImgFormat,
		allocation_size: u64,
	) -> ShmemDataInternal {
		ShmemDataInternal {
			name,
			handle_id,
			width,
			height,
			format,
			allocation_size,
		}
	}

	fn new_empty(img_name: &str) -> Result<ShmemDataInternal, Error> {
		if img_name.as_bytes().len() > size_of::<ImgName>() {
			Err(Error::new(
				ErrorKind::OutOfMemory,
				format!(
					"Image name '{img_name:?}' too long. Should be less than {:?}bytes",
					size_of::<ImgName>()
				),
			))
		} else {
			let shmem_internal = ShmemDataInternal {
				name: ImgData::convert_shmem_str_to_array(img_name),
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

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use raw_sync::Timeout;

	use crate::platform::img_data::ImgData;

	use super::IpcShmem;

	const SHMEM_NAME: &str = "shmem_name";
	const TIMEOUT: Timeout = Timeout::Val(Duration::from_secs(10));

	fn img_name() -> String {
		"img_name".to_string()
	}

	#[test]
	fn shmem_create() {
		IpcShmem::new(SHMEM_NAME, &img_name(), true).expect("Failed to create shmem");
	}

	fn _shmem_share() -> (IpcShmem, IpcShmem) {
		let created_shmem = IpcShmem::new(SHMEM_NAME, &img_name(), true).unwrap();
		let shared_shmem =
			IpcShmem::new(SHMEM_NAME, &img_name(), false).expect("Failed to share shmem");

		(created_shmem, shared_shmem)
	}

	#[test]
	fn shmem_name() {
		let shmem = IpcShmem::new(SHMEM_NAME, &img_name(), true).expect("Failed to create shmem");

		let rlock = shmem.acquire_rlock(TIMEOUT).unwrap();
		let rdata = IpcShmem::acquire_rdata(&rlock);

		assert_eq!(ImgData::convert_shmem_array_to_str(&rdata.name), img_name());
	}

	#[test]
	fn shmem_share() {
		let _ = shmem_create();
	}

	#[test]
	fn shmem_set_width() {
		const TEST_ORIG_VAL: u32 = 0;
		const TEST_CH_VAL: u32 = 12345;

		//let shared_shmem = IpcShmem::new(SHMEM_NAME, &img_name(), true).unwrap();

		let (created_shmem, shared_shmem) = _shmem_share();
		{
			let lock = shared_shmem.acquire_lock(TIMEOUT).unwrap();
			let data = IpcShmem::acquire_data(&lock);

			data.width = TEST_ORIG_VAL;
			assert_eq!(data.width, TEST_ORIG_VAL);

			data.width = TEST_CH_VAL;
		}

		{
			let rlock = created_shmem.acquire_rlock(TIMEOUT).unwrap();
			let rdata = IpcShmem::acquire_rdata(&rlock);

			assert_eq!(rdata.width, TEST_CH_VAL);
		}
	}
}
