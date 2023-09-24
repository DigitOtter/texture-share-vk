#include "texture_share_vk/texture_share_vk.h"

#include "texture_share_vk/platform/platform_vk.h"


TextureShareVk::~TextureShareVk()
{
	this->CleanupVulkan();
}

void TextureShareVk::InitializeVulkan()
{
	this->_vk_struct = VkHelpers::CreateTextureShareVkInstance();

	// Destroy vulkan if not importing
	this->_cleanup_vk = true;

	ExternalHandleVk::LoadVulkanHandleExtensions(this->_vk_struct.instance);
	ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(this->_vk_struct.physical_device);

	this->InitCommandBuffer();
}

void TextureShareVk::InitializeVulkan(VkInstance instance, VkDevice device,
                                      VkPhysicalDevice physical_device, VkQueue graphics_queue,
                                      uint32_t graphics_queue_index,
                                      bool import_only)
{
	this->_vk_struct = {};
	this->_vk_struct.instance = instance;
	this->_vk_struct.device = device;
	this->_vk_struct.physical_device = physical_device;
	this->_vk_struct.graphics_queue = graphics_queue;
	this->_vk_struct.graphics_queue_index = graphics_queue_index;

	// Destroy vulkan if not importing
	this->_cleanup_vk = !import_only;

	ExternalHandleVk::LoadVulkanHandleExtensions(this->_vk_struct.instance);
	ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(this->_vk_struct.physical_device);

	this->InitCommandBuffer();
}

void TextureShareVk::CleanupVulkan()
{
	if(this->_vk_struct.instance != VK_NULL_HANDLE)
	{
		VkHelpers::CleanupCommandBuffer(this->_vk_struct.device, this->_command_pool, this->_command_buffer);
		VkHelpers::CleanupCommandPool(this->_vk_struct.device, this->_command_pool);
		VkHelpers::CleanupTextureShareVkInstance(this->_vk_struct, this->_cleanup_vk, this->_cleanup_vk);

		this->_vk_struct.device = VK_NULL_HANDLE;
		this->_vk_struct.instance = VK_NULL_HANDLE;
	}
}

SharedImageVk TextureShareVk::CreateImage(uint32_t width, uint32_t height, uint64_t image_id, VkFormat format)
{
	SharedImageVk shared_image(this->_vk_struct.device);
	shared_image.Initialize(this->_vk_struct.device, this->_vk_struct.physical_device, width, height, image_id, format);
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

bool TextureShareVk::IsVulkanInitialized() const
{
	return this->_vk_struct.instance != VK_NULL_HANDLE;
}

void TextureShareVk::InitCommandBuffer()
{
	this->_command_pool = VkHelpers::CreateCommandPool(this->_vk_struct.device, this->_vk_struct.graphics_queue_index);
	this->_command_buffer = VkHelpers::CreateCommandBuffer(this->_vk_struct.device, this->_command_pool);
}
