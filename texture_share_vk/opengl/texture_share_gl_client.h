#ifndef TEXTURE_SHARE_GL_CLIENT_H
#define TEXTURE_SHARE_GL_CLIENT_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/platform/platform_gl.h"

#include <string>


class TextureShareGlClient
{
	public:
		TextureShareGlClient(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~TextureShareGlClient();

		void InitializeGlExt();
		void CleanupGl();

		static void InitDaemon(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                       const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data(),
		                       uint64_t wait_time_micro_s = DAEMON_STARTUP_DEFAULT_WAIT_TIME_MICRO_S);

		void InitImage(const std::string &image_name,
		               uint32_t image_width, uint32_t image_height,
		               VkFormat image_format);

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

};

#endif //TEXTURE_SHARE_GL_CLIENT_H
