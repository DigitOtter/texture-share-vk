use crate::platform::img_data::{ImgData, ImgFormat, ImgName};

use std::{
    mem::{size_of, ManuallyDrop},
    os::fd::RawFd,
};

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
    RenameImage,
    FindImage,
}

#[repr(C)]
pub union CommandData {
    pub init_img: ManuallyDrop<CommInitImage>,
    pub rename_img: ManuallyDrop<CommRenameImage>,
    pub find_img: ManuallyDrop<CommFindImage>,
}

#[repr(C)]
pub union ResultData {
    pub init_img: ManuallyDrop<ResultInitImage>,
    pub rename_img: ManuallyDrop<ResultRenameImage>,
    pub find_img: ManuallyDrop<ResultFindImage>,
}

pub struct CommInitImage {
    pub image_name: ImgName,
    pub width: u32,
    pub height: u32,
    pub format: ImgFormat,
}

pub struct ResultInitImage {
    pub shared_img_fd: RawFd,
    pub img_data: ImgData,
}

pub struct CommRenameImage {
    pub old_image_name: ImgName,
    pub new_image_name: ImgName,
}

pub struct ResultRenameImage {
    pub img_data: ImgData,
}

pub struct CommFindImage {
    pub image_name: ImgName,
}

pub struct ResultFindImage {
    pub shared_img_fd: RawFd,
    pub img_data: ImgData,
}

impl Default for CommandMsg {
    fn default() -> Self {
        Self {
            tag: CommandTag::FindImage,
            data: CommandData {
                find_img: ManuallyDrop::new(CommFindImage {
                    image_name: [0 as i8; size_of::<ImgName>()],
                }),
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
                    shared_img_fd: -1,
                    img_data: ImgData::default(),
                }),
            },
        }
    }
}
