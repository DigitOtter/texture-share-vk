#ifndef TEXTURE_SHARE_VK_CLIENT_H
#define TEXTURE_SHARE_VK_CLIENT_H

#include "texture_share_vk/ipc_memory/ipc_memory.h"
#include "texture_share_vk/texture_share_vk.h"

#include <map>

/*!
 * \brief Texture Share Client.
 * Connects to a central daemon and requests image data
 */
class TextureShareVkClient
{
	public:
	/*!
	 * \brief Constructor
	 * \param ipc_cmd_memory_segment Name of cmd memory segment. Should match daemon cmd memory name
	 * \param ipc_map_memory_segment Name of map memory segment. Should match daemon map memory name
	 */
	TextureShareVkClient(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
	                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
	~TextureShareVkClient();

	/*!
	 * \brief Initialize Vulkan. Loads Vulkan and required extensions
	 */
	void InitializeVulkan();

	/*!
	 * \brief Import Vulkan information
	 * \param import_only If true, does not clean up imported vulkan data on CleanupVulkan()
	 */
	void InitializeVulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
	                      VkQueue graphics_queue, uint32_t graphics_queue_index, bool import_only = true);

	/*!
	 * \brief Cleanup Vulkan. Closes local shared image handles.
	 * If requested on init, shuts down loaded vulkan data
	 */
	void CleanupVulkan();

	/*!
	 * \brief Init new shared image
	 * \param image_name Name of shared image
	 * \param image_width
	 * \param image_height
	 * \param image_format
	 * \param overwrite_existing If an image with the given name exists, should it be replaced?
	 * \param micro_sec_wait_time
	 */
	void InitImage(const std::string &image_name, uint32_t image_width, uint32_t image_height, VkFormat image_format,
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
	 * \param send_image Image to send
	 * \param send_image_layout Layout of send_image
	 * \param fence If set, use the given fence to synchronize execution
	 * \param micro_sec_wait_time
	 */
	void SendImageBlit(const std::string &image_name, VkImage send_image, VkImageLayout send_image_layout,
	                   VkFence fence = VK_NULL_HANDLE, uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Receive image via blit
	 * \param image_name Shared image name
	 * \param recv_image Image to receive to
	 * \param pre_recv_image_layout Layout of recv_image
	 * \param post_recv_image_layout Layout of recv_image after processing (usually the same as pre_recv_image_layout)
	 * \param fence If set, use the given fence to synchronize execution
	 * \param micro_sec_wait_time
	 */
	void RecvImageBlit(const std::string &image_name, VkImage recv_image, VkImageLayout pre_recv_image_layout,
	                   VkImageLayout post_recv_image_layout, VkFence fence = VK_NULL_HANDLE,
	                   uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Clear a shared image
	 * \param image_name Shared image name
	 * \param clear_color Clear color
	 * \param fence If set, use the given fence to synchronize execution
	 * \param micro_sec_wait_time
	 */
	void ClearImage(const std::string &image_name, VkClearColorValue clear_color, VkFence fence = VK_NULL_HANDLE,
	                uint64_t micro_sec_wait_time = IpcMemory::DEFAULT_CMD_WAIT_TIME);

	/*!
	 * \brief Access stored vulkan data
	 */
	constexpr const TextureShareVk &GetVulkanData() const
	{
		return this->_vk_data;
	}

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
	SharedImageHandleVk *SharedImageHandle(const std::string &image_name, bool update_data = false);

	private:
	/*!
	 * \brief Vulkan data
	 */
	TextureShareVk _vk_data;

	/*!
	 * \brief Ipc Memory Control
	 */
	IpcMemory _ipc_memory;

	struct SharedImageData
	{
		SharedImageHandleVk shared_image;
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

#endif // TEXTURE_SHARE_VK_CLIENT_H
