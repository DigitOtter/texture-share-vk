#include "texture_share_vk/texture_share_vk_client.h"

#include "texture_share_vk/platform/config.h"

#include <unistd.h>


void TextureShareVkClient::InitDaemon(const std::string &ipc_cmd_memory_segment,
                                      const std::string &ipc_map_memory_segment)
{
	if(IpcMemory::SharedMemoryExists(ipc_cmd_memory_segment))
		return;

	int c_pid = fork();
	if(c_pid == 0)
	{
		// Child process
		if(setsid() < 0)
			throw std::runtime_error("Failed to daemonize texture share daemon");

		const int ret = execlp(TSV_DAEMON_PATH,
		                       ipc_cmd_memory_segment.c_str(),
		                       ipc_map_memory_segment.c_str(),
		                       nullptr);
		exit(ret);
	}
	else if(c_pid < 0)
	{
		throw std::runtime_error("Failed to create texture share daemon");
	}
}

void TextureShareVkClient::InitImage(const std::string &image_name,
                                     uint32_t image_width, uint32_t image_height,
                                     VkFormat image_format)
{
	if(!this->_ipc_memory.SubmitWaitImageNameCmd(image_name, "",
	                                             image_width, image_height,
	                                             ExternalHandleVk::GetImageFormat(image_format)))
	{
		throw std::runtime_error("Failed to initialize shared image");
	}

	ExternalHandle::SharedImageInfo image_info = this->_ipc_memory.SubmitWaitExternalHandleCmd(image_name);
	this->_shared_image = this->_vk_data.CreateImageHandle(std::move(image_info));
}

void TextureShareVkClient::SendImageBlit(VkImage send_image, VkImageLayout send_image_layout, VkFence fence)
{



}
