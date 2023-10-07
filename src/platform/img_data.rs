use std::{ffi::CStr, mem::size_of, os::fd::RawFd};

pub(crate) type ImgName = [u8; 1024];
pub(crate) type ShmemName = [u8; 1024];

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImgFormat {
    R8G8B8A8,
    R8G8B8,
    B8G8R8A8,
    B8G8R8,
    Undefined,
}

#[repr(C)]
pub(crate) struct ImgData {
    pub name: ImgName,
    pub shmem_name: ShmemName,
    pub width: u32,
    pub height: u32,
    pub format: ImgFormat,
    pub allocation_size: u32,
}

impl ImgData {
    pub(crate) fn convert_shmem_str_to_array(shmem_name: &str) -> ShmemName {
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

    pub(crate) fn convert_shmem_array_to_str(shmem_name: &ShmemName) -> String {
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
            name: [0 as u8; size_of::<ImgName>()],
            shmem_name: [0 as u8; size_of::<ShmemName>()],
            width: 0,
            height: 0,
            format: ImgFormat::default(),
            allocation_size: 0,
        }
    }
}
