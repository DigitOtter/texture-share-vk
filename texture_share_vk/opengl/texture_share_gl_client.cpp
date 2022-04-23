#include "texture_share_gl_client.h"

#include <chrono>


namespace bipc = boost::interprocess;

TextureShareGlClient::TextureShareGlClient(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
    : _ipc_memory(IpcMemory::CreateIpcClientAndDaemon(ipc_cmd_memory_segment, ipc_map_memory_segment))
{}

void TextureShareGlClient::InitImage(const std::string &image_name,
                                     uint32_t image_width, uint32_t image_height,
                                     GLenum image_format,
                                     uint64_t micro_sec_wait_time)
{
	// Initialize image in daemon
	if(!this->_ipc_memory.SubmitWaitImageInitCmd(image_name,
	                                             image_width, image_height,
	                                             ExternalHandleGl::GetImageFormat(image_format),
	                                             micro_sec_wait_time))
	{
		throw std::runtime_error("Failed to initialize shared image");
	}

	// Receive image info from daemon
	ExternalHandle::SharedImageInfo image_info = this->_ipc_memory.SubmitWaitExternalHandleCmd(image_name, micro_sec_wait_time);
	sleep(1);
	this->_shared_image.InitializeWithExternal(std::move(image_info));

	// Get image sync data
	this->_ipc_img_data = this->_ipc_memory.GetImageData(image_name, micro_sec_wait_time);
}

void TextureShareGlClient::SendImageBlit(GLuint src_texture_id,
                                         GLuint src_texture_target, const ImageExtent &src_dimensions,
                                         bool invert, GLuint prev_fbo,
                                         uint64_t micro_sec_wait_time)
{
	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(this->_ipc_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	return this->_shared_image.RecvBlitImage(src_texture_id, src_texture_target, src_dimensions, invert, prev_fbo);
}

void TextureShareGlClient::RecvImageBlit(GLuint dst_texture_id,
                                         GLuint dst_texture_target, const ImageExtent &dst_dimensions,
                                         bool invert, GLuint prev_fbo,
                                         uint64_t micro_sec_wait_time)
{
	bipc::sharable_lock<bipc::interprocess_sharable_mutex> img_lock(this->_ipc_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	return this->_shared_image.RecvBlitImage(dst_texture_id, dst_texture_target, dst_dimensions, invert, prev_fbo);
}

void TextureShareGlClient::ClearImage(const void *clear_color, uint64_t micro_sec_wait_time)
{
	bipc::scoped_lock<bipc::interprocess_sharable_mutex> img_lock(this->_ipc_img_data->handle_access, bipc::try_to_lock);
	if(!img_lock)
	{
		if(!img_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return;
	}

	return this->_shared_image.ClearImage(clear_color);
}
