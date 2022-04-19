#include "texture_share_vk/texture_share_vk_client.h"

#include "texture_share_vk/platform/config.h"

#include <chrono>
#include <unistd.h>


namespace bipc = boost::interprocess;

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
	if(!this->_ipc_memory.SubmitWaitImageInitCmd(image_name,
	                                             image_width, image_height,
	                                             ExternalHandleVk::GetImageFormat(image_format)))
	{
		throw std::runtime_error("Failed to initialize shared image");
	}

	ExternalHandle::SharedImageInfo image_info = this->_ipc_memory.SubmitWaitExternalHandleCmd(image_name);
	this->_shared_image = this->_vk_data.CreateImageHandle(std::move(image_info));

	this->_img_data = this->_ipc_memory.GetImageData(image_name);
}

void TextureShareVkClient::SendImageBlit(VkImage send_image, VkImageLayout send_image_layout, VkFence fence, uint64_t micro_sec_wait_time)
{
	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(this->_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	this->_shared_image.SendImageBlit(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(),
	                                  send_image, send_image_layout,
	                                  fence);
}

void TextureShareVkClient::RecvImageBlit(VkImage recv_image, VkImageLayout recv_image_layout, VkFence fence, uint64_t micro_sec_wait_time)
{
	bipc::sharable_lock<bipc::interprocess_sharable_mutex> img_lock(this->_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	this->_shared_image.RecvImageBlit(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(),
	                                  recv_image, recv_image_layout,
	                                  fence);
}

void TextureShareVkClient::ClearImage(VkClearColorValue clear_color, VkFence fence, uint64_t micro_sec_wait_time)
{
	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(this->_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	this->_shared_image.ClearImage(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(),
	                               clear_color,
	                               fence);
}
