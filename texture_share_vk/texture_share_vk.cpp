#include "texture_share_vk/texture_share_vk.h"

#include "texture_share_vk/platform/platform_vk.h"


TextureShareVk::~TextureShareVk()
{
	this->CleanupVulkan();
}

void TextureShareVk::InitializeVulkan()
{
	this->_vk_struct = VkHelpers::CreateTextureShareVkInstance();
	this->_command_pool = VkHelpers::CreateCommandPool(this->_vk_struct.device, this->_vk_struct.graphics_queue_index);
	this->_command_buffer = VkHelpers::CreateCommandBuffer(this->_vk_struct.device, this->_command_pool);
}

void TextureShareVk::CleanupVulkan()
{
	if(this->_vk_struct.device != VK_NULL_HANDLE)
	{
		VkHelpers::CleanupCommandBuffer(this->_vk_struct.device, this->_command_pool, this->_command_buffer);
		VkHelpers::CleanupCommandPool(this->_vk_struct.device, this->_command_pool);
		VkHelpers::CleanupTextureShareVkInstance(this->_vk_struct);

		this->_vk_struct.device = VK_NULL_HANDLE;
	}
}

SharedImageVk TextureShareVk::CreateImage(uint32_t width, uint32_t height, VkFormat format)
{
	SharedImageVk shared_image(this->_vk_struct.device);
	shared_image.Initialize(this->_vk_struct.device, this->_vk_struct.physical_device, width, height, format);
	shared_image.InitializeImageLayout(this->_vk_struct.device, this->_vk_struct.graphics_queue, this->_command_buffer);

	return shared_image;
}

SharedImageHandleVk TextureShareVk::CreateImageHandle(ExternalHandle::SharedImageInfo &&image_info, VkImageLayout layout)
{
	SharedImageHandleVk shared_image_handle;
	shared_image_handle.ImportHandles(this->_vk_struct.device, this->_vk_struct.physical_device, std::move(image_info));

	shared_image_handle.SetImageLayout(this->_vk_struct.graphics_queue, this->_command_buffer, layout);

	return shared_image_handle;
}

SharedImageHandleVk TextureShareVk::CreateImageHandle(ExternalHandle::ShareHandles &&handles,
                                                      uint32_t width, uint32_t height,
                                                      VkFormat format, VkImageLayout layout)
{
	ExternalHandle::SharedImageInfo image_info;
	image_info.handles = std::move(handles);
	image_info.width = width;
	image_info.height = height;
	image_info.format = ExternalHandleVk::GetImageFormat(format);

	return this->CreateImageHandle(std::move(image_info), layout);
}
