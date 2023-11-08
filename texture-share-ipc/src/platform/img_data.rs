use std::mem::size_of;

use super::ShmemDataInternal;

pub(crate) type ImgName = [u8; 1024];
pub(crate) type ShmemName = [u8; 1024];

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImgFormat {
	R8G8B8A8,
	R8G8B8,
	B8G8R8A8,
	B8G8R8,
	Undefined,
}

#[repr(C)]
pub struct ImgData {
	pub shmem_name: ShmemName,
	pub data: ShmemDataInternal,
}

impl ImgData {
	pub fn new(
		shmem_name: ShmemName,
		image_name: ImgName,
		handle_id: u32,
		width: u32,
		height: u32,
		format: ImgFormat,
		allocation_size: u64,
	) -> ImgData {
		ImgData {
			shmem_name,
			data: ShmemDataInternal::new(
				image_name,
				handle_id,
				width,
				height,
				format,
				allocation_size,
			),
		}
	}

	pub fn from_shmem_data_internal(
		shmem_name: ShmemName,
		shmem_data_internal: ShmemDataInternal,
	) -> ImgData {
		ImgData {
			shmem_name,
			data: shmem_data_internal,
		}
	}

	pub fn convert_shmem_str_to_array(shmem_name: &str) -> ShmemName {
		// Generate ResultMsg data
		let shmem_name_array = shmem_name.as_bytes().to_owned();
		let shmem_name_len = shmem_name_array.len();
		if shmem_name_len >= size_of::<ShmemName>() {
			panic!("Shmem Name '{}' too long. Stopping", shmem_name);
		}

		let mut buf = [0 as u8; size_of::<ShmemName>()];
		buf[0..shmem_name_len].copy_from_slice(&shmem_name_array[0..shmem_name_len]);
		buf
	}

	pub fn convert_shmem_array_to_str(shmem_name: &ShmemName) -> String {
		let end = shmem_name.iter().position(|it| *it == 0 as u8);
		if end.is_none() {
			panic!(
				"Shmem Name '{}' contains no end",
				String::from_utf8_lossy(shmem_name)
			);
		}

		String::from_utf8_lossy(&shmem_name[0..end.unwrap()]).to_string()
	}
}

impl Default for ImgFormat {
	fn default() -> Self {
		ImgFormat::Undefined
	}
}

impl Default for ImgData {
	fn default() -> Self {
		Self {
			shmem_name: [0 as u8; size_of::<ShmemName>()],
			data: ShmemDataInternal::default(),
		}
	}
}
