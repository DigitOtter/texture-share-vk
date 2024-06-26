use crate::platform::img_data::{ImgData, ImgFormat, ImgName, ShmemName};

use std::mem::{size_of, ManuallyDrop};

#[repr(C)]
pub struct CommandMsg {
	pub tag: CommandTag,
	pub data: CommandData,
}

#[repr(C)]
pub struct ResultMsg {
	pub tag: CommandTag,
	pub data: ResultData,
}

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum CommandTag {
	InitImage,
	//RenameImage,
	FindImage,
	CopyImage,
}

#[repr(C)]
pub union CommandData {
	pub init_img: ManuallyDrop<CommInitImage>,
	pub find_img: ManuallyDrop<CommFindImage>,
	pub copy_img: ManuallyDrop<CommCopyImage>,
}

#[repr(C)]
pub union ResultData {
	pub init_img: ManuallyDrop<ResultInitImage>,
	pub find_img: ManuallyDrop<ResultFindImage>,
}

pub struct CommInitImage {
	pub image_name: ImgName,
	pub shmem_name: ShmemName,
	pub width: u32,
	pub height: u32,
	pub format: ImgFormat,
	pub overwrite_existing: bool,
	pub gpu_device_uuid: u128,
}

pub struct ResultInitImage {
	pub image_created: bool,
	pub img_data: ImgData,
}

pub struct CommRenameImage {
	pub old_image_name: ImgName,
	pub new_image_name: ImgName,
}

pub struct ResultRenameImage {
	pub image_found: bool,
	pub img_data: ImgData,
}

pub struct CommFindImage {
	pub image_name: ImgName,
	pub gpu_device_uuid: u128,
}

pub struct ResultFindImage {
	pub image_found: bool,
	pub img_data: ImgData,
}

pub struct CommCopyImage {
	pub image_name: ImgName,
	pub gpu_device_uuid: u128,
}

impl Default for CommandMsg {
	fn default() -> Self {
		Self {
			tag: CommandTag::FindImage,
			data: CommandData {
				find_img: ManuallyDrop::new(CommFindImage::default()),
			},
		}
	}
}

impl Default for ResultMsg {
	fn default() -> Self {
		Self {
			tag: CommandTag::FindImage,
			data: ResultData {
				find_img: ManuallyDrop::new(ResultFindImage {
					image_found: false,
					img_data: ImgData::default(),
				}),
			},
		}
	}
}

impl Default for CommInitImage {
	fn default() -> Self {
		CommInitImage {
			image_name: [0 as u8; size_of::<ImgName>()],
			shmem_name: [0 as u8; size_of::<ShmemName>()],
			format: ImgFormat::default(),
			width: 0,
			height: 0,
			overwrite_existing: false,
			gpu_device_uuid: uuid::Uuid::nil().as_u128(),
		}
	}
}

impl Default for CommFindImage {
	fn default() -> Self {
		Self {
			image_name: [0 as u8; size_of::<ImgName>()],
			gpu_device_uuid: uuid::Uuid::nil().as_u128(),
		}
	}
}

impl Default for CommCopyImage {
	fn default() -> Self {
		Self {
			image_name: [0 as u8; size_of::<ImgName>()],
			gpu_device_uuid: uuid::Uuid::nil().as_u128(),
		}
	}
}
