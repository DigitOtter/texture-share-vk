#ifndef SHARED_IMAGE_HANDLE_VK_H
#define SHARED_IMAGE_HANDLE_VK_H

#include "texture_share_vk/platform/platform_vk.h"

#include <functional>

/*!
 * \brief Manages imported image in Vulkan
 */
class SharedImageHandleVk
{
	public:
	SharedImageHandleVk() = default;
	~SharedImageHandleVk();

	SharedImageHandleVk(const SharedImageHandleVk &)            = delete;
	SharedImageHandleVk &operator=(const SharedImageHandleVk &) = delete;

	SharedImageHandleVk(SharedImageHandleVk &&) = default;
	SharedImageHandleVk &operator=(SharedImageHandleVk &&);


	/*!
	 * \brief Import image from external handles
	 * \param external_handles External Handles. Ownership of handles is transferred to vulkan on import
	 */
	/*!
	 * \brief Import image from external handles
	 * \param device Device
	 * \param physical_device Physical device
	 * \param external_handles External Handles. Ownership of handles is transferred to vulkan on import
	 */
	void ImportHandles(VkDevice device, VkPhysicalDevice physical_device,
	                   ExternalHandle::SharedImageInfo &&external_handles);

	/*!
	 * \brief Set shared image layout
	 * \param graphics_queue Graphics queue to use
	 * \param command_buffer Command buffer to use
	 * \param image_layout New Image layout
	 */
	void SetImageLayout(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImageLayout image_layout);

	void SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image,
	                   VkImageLayout send_image_layout, VkFence fence);
	void SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image,
	                   VkImageLayout send_image_layout, VkFence fence, const VkOffset3D send_image_extent[2]);
	void RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image,
	                   VkImageLayout pre_recv_image_layout, VkImageLayout post_recv_image_layout, VkFence fence);
	void RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image,
	                   VkImageLayout pre_recv_image_layout, VkImageLayout post_recv_image_layout, VkFence fence,
	                   const VkOffset3D recv_image_extent[2]);

	void ClearImage(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkClearColorValue clear_color,
	                VkFence fence);

	void Cleanup();

	static VkImageSubresourceLayers CreateColorSubresourceLayer();

	constexpr VkFormat ImageFormat() const
	{
		return this->_format;
	}

	constexpr VkImage TextureId() const
	{
		return this->_image;
	}

	constexpr uint32_t Width() const
	{
		return this->_width;
	}

	constexpr uint32_t Height() const
	{
		return this->_height;
	}

	constexpr uint64_t HandleId() const
	{
		return this->_handle_id;
	}

	private:
	VkDevice _device{VK_NULL_HANDLE};
	VkImage _image{VK_NULL_HANDLE};
	VkDeviceMemory _image_memory{VK_NULL_HANDLE};

	VkSemaphore _semaphore_read{VK_NULL_HANDLE};
	VkSemaphore _semaphore_write{VK_NULL_HANDLE};

	VkImageLayout _image_layout = VK_IMAGE_LAYOUT_UNDEFINED;

	uint32_t _width;
	uint32_t _height;
	VkFormat _format;
	uint64_t _handle_id = 0;

	static VkSemaphore ImportSemaphoreHandle(VkDevice device, ExternalHandle::TYPE semaphore_handle);
};

#endif // SHARED_IMAGE_HANDLE_VK_H
