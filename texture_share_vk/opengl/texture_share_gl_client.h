#ifndef TEXTURE_SHARE_GL_CLIENT_H
#define TEXTURE_SHARE_GL_CLIENT_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/opengl/shared_image_handle_gl.h"
#include "texture_share_vk/platform/platform_gl.h"

#include <map>
#include <string>


class TextureShareGlClient
{
	public:
		using ImageExtent = SharedImageHandleGl::ImageExtent;

		TextureShareGlClient(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~TextureShareGlClient() = default;

		void InitializeGlExt();
		void CleanupGl();

		void InitImage(const std::string &image_name,
		               uint32_t image_width, uint32_t image_height, GLenum image_format,
		               bool overwrite_existing = false,
		               uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);
		bool FindImage(const std::string &image_name,
		               uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		void SendImageBlit(const std::string &image_name,
		                   GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions,
		                   bool invert = false, GLuint prev_fbo = 0,
		                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);
		void RecvImageBlit(const std::string &image_name,
		                   GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions,
		                   bool invert = false, GLuint prev_fbo = 0,
		                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		void ClearImage(const std::string &image_name, const void *clear_color,
		                uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		SharedImageHandleGl *SharedImageHandle(const std::string &image_name);

	private:
		IpcMemory _ipc_memory;

		struct SharedImageData
		{
			SharedImageHandleGl shared_image;
			IpcMemory::ImageData *ipc_img_data = nullptr;
		};

		std::map<std::string, SharedImageData> _shared_image_data;

		SharedImageData *FindImageInternal(const std::string &image_name,
		                                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

		SharedImageData *GetImageData(const std::string &image_name,
		                              uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);
};

#endif //TEXTURE_SHARE_GL_CLIENT_H
