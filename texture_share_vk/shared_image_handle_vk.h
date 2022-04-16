#ifndef SHARED_IMAGE_HANDLE_VK_H
#define SHARED_IMAGE_HANDLE_VK_H

#include "texture_share_vk/platform/platform_vk.h"


/*!
 * \brief Manages imported image in Vulkan
 */
class SharedImageHandleVk
{
	public:
		SharedImageHandleVk() = default;
		~SharedImageHandleVk();

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
		void ImportHandles(VkDevice device, VkPhysicalDevice physical_device, ExternalHandle::SharedImageInfo &&external_handles);

		/*!
		 * \brief Set shared image layout
		 * \param graphics_queue Graphics queue to use
		 * \param command_buffer Command buffer to use
		 * \param image_layout New Image layout
		 */
		void SetImageLayout(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImageLayout image_layout);

		void SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer,
		                   VkImage send_image, VkImageLayout send_image_layout,
		                   VkFence fence);
		void RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer,
		                   VkImage recv_image, VkImageLayout recv_image_layout,
		                   VkFence fence);

		void ClearImage(VkQueue graphics_queue, VkCommandBuffer command_buffer,
		                VkClearColorValue clear_color,
		                VkFence fence);


		void SendImageBlitCmd(VkCommandBuffer command_buffer, VkImage send_image, VkImageLayout send_image_layout);
		void ReceiveImageBlitCmd(VkCommandBuffer command_buffer, VkImage recv_image, VkImageLayout recv_image_layout);

		void ClearImageCmd(VkCommandBuffer command_buffer, VkClearColorValue clear_color);

		void Cleanup();

		static VkImageSubresourceLayers CreateColorSubresourceLayer();

	private:
		VkDevice _device              {VK_NULL_HANDLE};
		VkImage _image                {VK_NULL_HANDLE};
		VkDeviceMemory _image_memory  {VK_NULL_HANDLE};

		VkSemaphore _semaphore_read   {VK_NULL_HANDLE};
		VkSemaphore _semaphore_write  {VK_NULL_HANDLE};

		VkImageLayout _image_layout = VK_IMAGE_LAYOUT_UNDEFINED;

		uint32_t _width;
		uint32_t _height;
		VkFormat _format;

		static VkSemaphore ImportSemaphoreHandle(VkDevice device, ExternalHandle::TYPE semaphore_handle);

		//using transceive_fcn_t = void(SharedImageHandleVk::*)(VkCommandBuffer,VkImage,VkImageLayout);
		using transceive_fcn_t = std::function<void()>;
		void TransceiveImageRecordCmdBuf(VkCommandBuffer command_buffer,
		                                 //VkImage transceive_image, VkImageLayout transceive_image_layout,
		                                 VkImageLayout shared_image_requested_layout,
		                                 transceive_fcn_t f);

		void SubmitCommandBuffer(VkQueue graphics_queue, VkCommandBuffer command_buffer,
		                         VkSemaphore *wait_semaphores, uint32_t num_wait_semaphores,
		                         VkSemaphore *signal_semaphores, uint32_t num_signal_semaphores,
		                         VkFence fence = VK_NULL_HANDLE);
};

#endif // SHARED_IMAGE_HANDLE_VK_H
