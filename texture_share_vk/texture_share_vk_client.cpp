#include "texture_share_vk/texture_share_vk_client.h"

#include "texture_share_vk/platform/config.h"

#include "texture_share_vk/platform/daemon_comm.h"

#include <chrono>
#include <thread>


namespace bipc = boost::interprocess;

TextureShareVkClient::TextureShareVkClient(const std::string &ipc_cmd_memory_segment,
                                           const std::string &ipc_map_memory_segment)
	: _ipc_memory(IpcMemory::CreateIpcClientAndDaemon(ipc_cmd_memory_segment, ipc_map_memory_segment))
{}

TextureShareVkClient::~TextureShareVkClient()
{
	// TODO: Remove reference to image from daemon (?)
}

void TextureShareVkClient::InitializeVulkan()
{
	this->_vk_data.InitializeVulkan();
}

void TextureShareVkClient::InitializeVulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
                                            VkQueue graphics_queue, uint32_t graphics_queue_index, bool import_only)
{
	this->_vk_data.InitializeVulkan(instance, device, physical_device, graphics_queue, graphics_queue_index,
	                                import_only);
}

void TextureShareVkClient::CleanupVulkan()
{
	for(auto &img_data: this->_shared_image_data)
	{
		img_data.second.shared_image.Cleanup();
	}
	this->_shared_image_data.clear();

	this->_vk_data.CleanupVulkan();
}

void TextureShareVkClient::InitImage(const std::string &image_name, uint32_t image_width, uint32_t image_height,
                                     VkFormat image_format, bool overwrite_existing, uint64_t micro_sec_wait_time)
{
	if(!this->_ipc_memory.SubmitWaitImageInitCmd(image_name, image_width, image_height,
	                                             ExternalHandleVk::GetImageFormat(image_format), overwrite_existing,
	                                             micro_sec_wait_time))
	{
		throw std::runtime_error("Failed to initialize shared image");
	}

	if(!this->FindImage(image_name, micro_sec_wait_time))
		throw std::runtime_error("Failed to retrieve image handles after initialization");
}

bool TextureShareVkClient::FindImage(const std::string &image_name, uint64_t micro_sec_wait_time)
{
	return this->FindImageInternal(image_name, micro_sec_wait_time) != nullptr;
}

void TextureShareVkClient::SendImageBlit(const std::string &image_name, VkImage send_image,
                                         VkImageLayout send_image_layout, VkFence fence, uint64_t micro_sec_wait_time)
{
	SharedImageData *img_data = this->GetImageData(image_name, false, micro_sec_wait_time);
	if(!img_data)
		return;

	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(img_data->ipc_img_data->handle_access,
	                                                              bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	img_data->shared_image.SendImageBlit(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(), send_image,
	                                     send_image_layout, fence);
}

void TextureShareVkClient::RecvImageBlit(const std::string &image_name, VkImage recv_image,
                                         VkImageLayout pre_recv_image_layout, VkImageLayout post_recv_image_layout,
                                         VkFence fence, uint64_t micro_sec_wait_time)
{
	SharedImageData *img_data = this->GetImageData(image_name, false, micro_sec_wait_time);
	if(!img_data || !img_data->ipc_img_data)
		return;

	bipc::sharable_lock<bipc::interprocess_sharable_mutex> img_lock(img_data->ipc_img_data->handle_access,
	                                                                bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	img_data->shared_image.RecvImageBlit(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(), recv_image,
	                                     pre_recv_image_layout, post_recv_image_layout, fence);
}

void TextureShareVkClient::ClearImage(const std::string &image_name, VkClearColorValue clear_color, VkFence fence,
                                      uint64_t micro_sec_wait_time)
{
	SharedImageData *img_data = this->GetImageData(image_name, false, micro_sec_wait_time);
	if(!img_data)
		return;

	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(img_data->ipc_img_data->handle_access,
	                                                              bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	img_data->shared_image.ClearImage(this->_vk_data.GraphicsQueue(), this->_vk_data.CommandBuffer(), clear_color,
	                                  fence);
}

bool TextureShareVkClient::HasImageMemoryChanged(const std::string &image_name)
{
	if(auto img_it = this->_shared_image_data.find(image_name); img_it != this->_shared_image_data.end())
	{
		const SharedImageHandleVk &loc_img_data  = img_it->second.shared_image;
		const IpcMemory::ImageData *ipc_img_data = this->_ipc_memory.GetImageData(image_name);
		if(!ipc_img_data)
			return true;

		// Compare local storage to ipc memory
		if(ipc_img_data->shared_image_info.handle_id != loc_img_data.HandleId())
			return true;
	}

	return false;
}

SharedImageHandleVk *TextureShareVkClient::SharedImageHandle(const std::string &image_name, bool update_data)
{
	SharedImageData *img_data = this->GetImageData(image_name, update_data);
	return img_data != nullptr ? &img_data->shared_image : nullptr;
}

TextureShareVkClient::SharedImageData *TextureShareVkClient::FindImageInternal(const std::string &image_name,
                                                                               uint64_t micro_sec_wait_time)
{
	// Receive image info from daemon
	ExternalHandle::SharedImageInfo image_info =
		this->_ipc_memory.SubmitWaitExternalHandleCmd(image_name, micro_sec_wait_time);
	if(image_info.handles.memory == ExternalHandle::INVALID_VALUE ||
	   image_info.handles.ext_read == ExternalHandle::INVALID_VALUE ||
	   image_info.handles.ext_write == ExternalHandle::INVALID_VALUE)
		return nullptr;

	auto res = this->_shared_image_data.try_emplace(image_name, SharedImageData());
	if(!res.second)
	{
		// Cleanup old image data if already in memory
		res.first->second.shared_image.Cleanup();
	}

	res.first->second.shared_image = this->_vk_data.CreateImageHandle(std::move(image_info));

	// Get image sync data
	res.first->second.ipc_img_data = this->_ipc_memory.GetImageData(image_name, micro_sec_wait_time);

	return &res.first->second;
}

TextureShareVkClient::SharedImageData *TextureShareVkClient::GetImageData(const std::string &image_name,
                                                                          bool update_data,
                                                                          uint64_t micro_sec_wait_time)
{
	if(!update_data)
	{
		// Check local storage
		if(auto img_it = this->_shared_image_data.find(image_name); img_it != this->_shared_image_data.end())
			return &img_it->second;
	}

	// Check shared storage
	return this->FindImageInternal(image_name, micro_sec_wait_time);
}
