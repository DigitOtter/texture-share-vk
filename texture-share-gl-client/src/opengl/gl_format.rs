use std::fmt::Debug;

use texture_share_ipc::platform::img_data::ImgFormat;

use super::gl_shared_image::ffi::GlFormat;

impl From<ImgFormat> for GlFormat {
    fn from(value: ImgFormat) -> Self {
        match value {
            ImgFormat::B8G8R8 => GlFormat::BGR,
            ImgFormat::B8G8R8A8 => GlFormat::BGRA,
            ImgFormat::R8G8B8 => GlFormat::RGB,
            ImgFormat::R8G8B8A8 => GlFormat::RGBA,
            ImgFormat::Undefined => GlFormat::FALSE,
        }
    }
}

impl Into<ImgFormat> for GlFormat {
    fn into(self) -> ImgFormat {
        match self {
            GlFormat::BGR => ImgFormat::B8G8R8,
            GlFormat::BGRA => ImgFormat::B8G8R8A8,
            GlFormat::RGB => ImgFormat::R8G8B8,
            GlFormat::RGBA => ImgFormat::R8G8B8A8,
            GlFormat::FALSE => ImgFormat::Undefined,
            _ => panic!("VkFormat {:?} not implemented", self),
        }
    }
}
