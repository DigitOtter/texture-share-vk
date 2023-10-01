use std::{mem::size_of, os::fd::RawFd};

pub type ImgName = [i8; 1024];

#[repr(u32)]
pub enum ImgFormat {
    R8G8B8A8,
    R8G8B8,
    B8G8R8A8,
    B8G8R8,
    Undefined,
}

#[repr(C)]
pub struct ImgData {
    pub name: ImgName,
    pub handle_id: RawFd,
    pub width: u32,
    pub height: u32,
    pub format: ImgFormat,
    pub allocation_size: u32,
}

impl Default for ImgFormat {
    fn default() -> Self {
        ImgFormat::Undefined
    }
}

impl Default for ImgData {
    fn default() -> Self {
        Self {
            name: [0 as i8; size_of::<ImgName>()],
            handle_id: -1,
            width: 0,
            height: 0,
            format: ImgFormat::default(),
            allocation_size: 0,
        }
    }
}
