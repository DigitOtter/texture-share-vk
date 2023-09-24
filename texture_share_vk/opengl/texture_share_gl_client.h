#ifndef TEXTURE_SHARE_GL_CLIENT_H
#define TEXTURE_SHARE_GL_CLIENT_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/opengl/shared_image_handle_gl.h"
#include "texture_share_vk/platform/platform_gl.h"

#include <map>
#include <string>

/*!
 * \brief Texture Share Client.
 * Connects to a central daemon and requests image data
 */
class TextureShareGlClient
{
	public:
	using ImageExtent = SharedImageHandleGl::ImageExtent;

	/*!
	 * \brief Constructor
	 * \param ipc_cmd_memory_segment Name of cmd memory segment. Should match daemon cmd memory name
	 * \param ipc_map_memory_segment Name of map memory segment. Should match daemon map memory name
	 */
	TextureShareGlClient(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
	                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
	~TextureShareGlClient() = default;

	// void InitializeGlExt();
	// void CleanupGl();

	/*!
	 * \brief Init new shared image
	 * \param image_name Name of shared image
	 * \param image_width
	 * \param image_height
	 * \param image_format
	 * \param overwrite_existing If an image with the given name exists, should it be replaced?
	 * \param micro_sec_wait_time
	 */
	void InitImage(const std::string &image_name, uint32_t image_width, uint32_t image_height, GLenum image_format,
	               bool overwrite_existing = false, uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Find an existin shared image
	 * \param image_name Name of shared image
	 * \param micro_sec_wait_time
	 * \return Returns true if image found, false otherwise
	 */
	bool FindImage(const std::string &image_name, uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Send image via blit
	 * \param image_name Shared image name
	 * \param src_texture_id Texture Id to send
	 * \param src_texture_target Target of src_texture_id. Usually GL_TEXTURE_2D
	 * \param src_dimensions Texture dimenstions
	 * \param invert Should image be flipped upside down on send?
	 * \param prev_fbo Previous fbo that should be restored before returning from function
	 * \param micro_sec_wait_time
	 */
	void SendImageBlit(const std::string &image_name, GLuint src_texture_id, GLuint src_texture_target,
	                   const ImageExtent &src_dimensions, bool invert = false, GLuint prev_fbo = 0,
	                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Receive image via blit
	 * \param image_name Shared image name
	 * \param dst_texture_id Texture Id to receive to
	 * \param dst_texture_target Target of dst_texture_id. Usually GL_TEXTURE_2D
	 * \param dst_dimensions Texture dimenstions
	 * \param invert Should image be flipped upside down on send?
	 * \param prev_fbo Previous fbo that should be restored before returning from function
	 * \param micro_sec_wait_time
	 */
	void RecvImageBlit(const std::string &image_name, GLuint dst_texture_id, GLuint dst_texture_target,
	                   const ImageExtent &dst_dimensions, bool invert = false, GLuint prev_fbo = 0,
	                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Clear image
	 * \param image_name Shared image name
	 * \param clear_color Clear color. Struture depends on shared image format. For GL_RGBA, clear_color should be in
	 * the form of u_char[4] \param micro_sec_wait_time
	 */
	void ClearImage(const std::string &image_name, const void *clear_color,
	                uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Has the memory footprint changed? If yes, texture must be reloaded with FindImage(...)
	 * \param image_name Shared image name
	 */
	bool HasImageMemoryChanged(const std::string &image_name);

	/*!
	 * \brief Directly access shared image handle. Only use this if you know what you're doing
	 * \param image_name Shared image name
	 * \param update_data If true, retrieve updated data from shared memory
	 */
	SharedImageHandleGl *SharedImageHandle(const std::string &image_name, bool update_data = false);

	private:
	/*!
	 * \brief Ipc Memory Control
	 */
	IpcMemory _ipc_memory;

	struct SharedImageData
	{
		SharedImageHandleGl shared_image;
		IpcMemory::ImageData *ipc_img_data = nullptr;
	};

	/*!
	 * \brief Stored local image handles
	 */
	std::map<std::string, SharedImageData> _shared_image_data;

	/*!
	 * \brief Internal find image. Calls daemon to retrieve image
	 * \param image_name Shared image name
	 * \param micro_sec_wait_time
	 * \return Returns image data on success, or nullptr if image_name doesn't exist
	 */
	SharedImageData *FindImageInternal(const std::string &image_name,
	                                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Searches local _shared_image_data for image_name. If not found, tries to retrieve image from daemon
	 * \param image_name Shared image name
	 * \param update_data If true, retrieve updated data from shared memory
	 * \param micro_sec_wait_time
	 * \return Returns image data on success, or nullptr if image_name doesn't exist
	 */
	SharedImageData *GetImageData(const std::string &image_name, bool update_data = false,
	                              uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);
};

#endif // TEXTURE_SHARE_GL_CLIENT_H
