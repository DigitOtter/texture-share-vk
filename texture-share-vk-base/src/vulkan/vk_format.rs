use super::vk_shared_image::ffi::VkFormat;
use texture_share_ipc::platform::img_data::ImgFormat;

impl From<ImgFormat> for VkFormat {
	fn from(value: ImgFormat) -> Self {
		match value {
			ImgFormat::B8G8R8 => VkFormat::VK_FORMAT_B8G8R8_UNORM,
			ImgFormat::B8G8R8A8 => VkFormat::VK_FORMAT_B8G8R8A8_UNORM,
			ImgFormat::R8G8B8 => VkFormat::VK_FORMAT_R8G8B8_UNORM,
			ImgFormat::R8G8B8A8 => VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
			ImgFormat::Undefined => VkFormat::VK_FORMAT_UNDEFINED,
		}
	}
}

impl Into<ImgFormat> for VkFormat {
	fn into(self) -> ImgFormat {
		match self {
			VkFormat::VK_FORMAT_B8G8R8_UNORM => ImgFormat::B8G8R8,
			VkFormat::VK_FORMAT_B8G8R8A8_UNORM => ImgFormat::B8G8R8A8,
			VkFormat::VK_FORMAT_R8G8B8_UNORM => ImgFormat::R8G8B8,
			VkFormat::VK_FORMAT_R8G8B8A8_UNORM => ImgFormat::R8G8B8A8,
			VkFormat::VK_FORMAT_UNDEFINED => ImgFormat::Undefined,
			_ => panic!("VkFormat {:?} not implemented", self),
		}
	}
}
