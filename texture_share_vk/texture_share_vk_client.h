#ifndef TEXTURE_SHARE_VK_CLIENT_H
#define TEXTURE_SHARE_VK_CLIENT_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/texture_share_vk.h"


class TextureShareVkClient
{
	public:
		TextureShareVkClient(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~TextureShareVkClient();

		void InitializeVulkan();
		void InitializeVulkan(VkInstance instance, VkDevice device,
		                      VkPhysicalDevice physical_device, VkQueue graphics_queue,
		                      uint32_t graphics_queue_index,
		                      bool import_only = true);
		void CleanupVulkan();

		void InitImage(const std::string &image_name,
		               uint32_t image_width, uint32_t image_height,
		               VkFormat image_format,
		               uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		void SendImageBlit(VkImage send_image, VkImageLayout send_image_layout,
		                   VkFence fence = VK_NULL_HANDLE,
		                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);
		void RecvImageBlit(VkImage recv_image, VkImageLayout recv_image_layout,
		                   VkFence fence = VK_NULL_HANDLE,
		                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		void ClearImage(VkClearColorValue clear_color,
		                VkFence fence = VK_NULL_HANDLE,
		                uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		constexpr const TextureShareVk &GetVulkanData() const
		{	return this->_vk_data;	}

		constexpr SharedImageHandleVk &SharedImageHandle()
		{	return this->_shared_image;	}

	private:
		TextureShareVk _vk_data;
		SharedImageHandleVk _shared_image;

		IpcMemory _ipc_memory;
		IpcMemory::ImageData *_img_data = nullptr;
};

#endif //TEXTURE_SHARE_VK_CLIENT_H
