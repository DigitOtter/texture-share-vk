#ifndef TEXTURE_SHARE_VK_H
#define TEXTURE_SHARE_VK_H

#include "texture_share_vk/shared_image_handle_vk.h"
#include "texture_share_vk/shared_image_vk.h"
#include "texture_share_vk/vk_helpers.h"


class TextureShareVk
{
	public:
		static constexpr VkFormat DEFAULT_FORMAT = VK_FORMAT_R8G8B8A8_UNORM;

		TextureShareVk() = default;
		~TextureShareVk();

		void InitializeVulkan();
		void InitializeVulkan(VkInstance instance, VkDevice device,
		                      VkPhysicalDevice physical_device, VkQueue graphics_queue,
		                      uint32_t graphics_queue_index,
		                      bool import_only = true);
		void CleanupVulkan();

		SharedImageVk CreateImage(uint32_t width, uint32_t height, VkFormat format = DEFAULT_FORMAT);
		SharedImageHandleVk CreateImageHandle(ExternalHandle::SharedImageInfo &&image_info, VkImageLayout layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL);
		SharedImageHandleVk CreateImageHandle(ExternalHandle::ShareHandles &&handles,
		                                      uint32_t width, uint32_t height,
		                                      VkFormat format = DEFAULT_FORMAT,
		                                      VkImageLayout layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL);

		bool IsVulkanInitialized() const;

		constexpr VkInstance VulkanInstance()
		{	return this->_vk_struct.instance;	}

		constexpr VkQueue GraphicsQueue()
		{	return this->_vk_struct.graphics_queue;	}

		constexpr VkCommandBuffer CommandBuffer()
		{	return this->_command_buffer;	}

	private:
		VkHelpers::TextureShareVkStruct _vk_struct{};
		bool _cleanup_vk = true;

		VkCommandPool _command_pool{VK_NULL_HANDLE};
		VkCommandBuffer _command_buffer{VK_NULL_HANDLE};

		void InitCommandBuffer();
};

#endif //TEXTURE_SHARE_VK_H
