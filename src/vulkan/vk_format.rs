use crate::platform::img_data::ImgFormat;
use vulkano;

pub type VkImgFormat = vulkano::format::Format;

pub fn convert_vk_to_img_format(format: VkImgFormat) -> Option<ImgFormat> {
	match format {
		VkImgFormat::R8G8B8A8_UNORM => Some(ImgFormat::B8G8R8A8),
		VkImgFormat::R8G8B8_UNORM => Some(ImgFormat::R8G8B8),
		VkImgFormat::B8G8R8A8_UNORM => Some(ImgFormat::B8G8R8A8),
		VkImgFormat::B8G8R8_UNORM => Some(ImgFormat::B8G8R8),
		_ => None,
	}
}

pub fn convert_img_to_vkformat(format: ImgFormat) -> Option<VkImgFormat> {
	match format {
		ImgFormat::R8G8B8A8 => Some(VkImgFormat::R8G8B8A8_UNORM),
		ImgFormat::R8G8B8 => Some(VkImgFormat::R8G8B8_UNORM),
		ImgFormat::B8G8R8A8 => Some(VkImgFormat::B8G8R8A8_UNORM),
		ImgFormat::B8G8R8 => Some(VkImgFormat::B8G8R8_UNORM),
		_ => None,
	}
}
