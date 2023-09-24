#include "texture_share_gl_client.h"

#include <chrono>


namespace bipc = boost::interprocess;

TextureShareGlClient::TextureShareGlClient(const std::string &ipc_cmd_memory_segment,
                                           const std::string &ipc_map_memory_segment)
	: _ipc_memory(IpcMemory::CreateIpcClientAndDaemon(ipc_cmd_memory_segment, ipc_map_memory_segment))
{}

void TextureShareGlClient::InitImage(const std::string &image_name, uint32_t image_width, uint32_t image_height,
                                     GLenum image_format, bool overwrite_existing, uint64_t micro_sec_wait_time)
{
	// Initialize image in daemon
	if(!this->_ipc_memory.SubmitWaitImageInitCmd(image_name, image_width, image_height,
	                                             ExternalHandleGl::GetImageFormat(image_format), overwrite_existing,
	                                             micro_sec_wait_time))
	{
		throw std::runtime_error("Failed to initialize shared image");
	}

	// Receive image info from daemon
	if(!this->FindImage(image_name, micro_sec_wait_time))
		throw std::runtime_error("Failed to retrieve image handles after initialization");
}

bool TextureShareGlClient::FindImage(const std::string &image_name, uint64_t micro_sec_wait_time)
{
	return this->FindImageInternal(image_name, micro_sec_wait_time) != nullptr;
}

void TextureShareGlClient::SendImageBlit(const std::string &image_name, GLuint src_texture_id,
                                         GLuint src_texture_target, const ImageExtent &src_dimensions, bool invert,
                                         GLuint prev_fbo, uint64_t micro_sec_wait_time)
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

	return img_data->shared_image.SendBlitImage(src_texture_id, src_texture_target, src_dimensions, invert, prev_fbo);
}

void TextureShareGlClient::RecvImageBlit(const std::string &image_name, GLuint dst_texture_id,
                                         GLuint dst_texture_target, const ImageExtent &dst_dimensions, bool invert,
                                         GLuint prev_fbo, uint64_t micro_sec_wait_time)
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

	return img_data->shared_image.RecvBlitImage(dst_texture_id, dst_texture_target, dst_dimensions, invert, prev_fbo);
}

void TextureShareGlClient::ClearImage(const std::string &image_name, const void *clear_color,
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

	return img_data->shared_image.ClearImage((u_char *)clear_color);
}

bool TextureShareGlClient::HasImageMemoryChanged(const std::string &image_name)
{
	if(auto img_it = this->_shared_image_data.find(image_name); img_it != this->_shared_image_data.end())
	{
		const SharedImageHandleGl &loc_img_data  = img_it->second.shared_image;
		const IpcMemory::ImageData *ipc_img_data = this->_ipc_memory.GetImageData(image_name);
		if(!ipc_img_data)
			return true;

		// Compare local storage to ipc memory
		if(ipc_img_data->shared_image_info.handle_id != loc_img_data.HandleId())
			return true;
	}

	return false;
}

SharedImageHandleGl *TextureShareGlClient::SharedImageHandle(const std::string &image_name, bool update_data)
{
	SharedImageData *img_data = this->GetImageData(image_name, false, update_data);
	return img_data != nullptr ? &img_data->shared_image : nullptr;
}

TextureShareGlClient::SharedImageData *TextureShareGlClient::FindImageInternal(const std::string &image_name,
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

	res.first->second.shared_image.InitializeWithExternal(std::move(image_info));

	// Get image sync data
	res.first->second.ipc_img_data = this->_ipc_memory.GetImageData(image_name, micro_sec_wait_time);

	return &res.first->second;
}

TextureShareGlClient::SharedImageData *TextureShareGlClient::GetImageData(const std::string &image_name,
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
