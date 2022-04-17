#ifndef SHARED_IMAGE_VK_H
#define SHARED_IMAGE_VK_H

#include "texture_share_vk/platform/platform_vk.h"


class SharedImageVk
{
	public:
		SharedImageVk(VkDevice device = VK_NULL_HANDLE);
		~SharedImageVk();

		SharedImageVk(const SharedImageVk &other) = delete;
		SharedImageVk &operator=(const SharedImageVk &other) = delete;

		SharedImageVk(SharedImageVk &&other);
		SharedImageVk &operator=(SharedImageVk &&other);

		void Initialize(VkDevice device, VkPhysicalDevice physical_device,
		                uint32_t image_width, uint32_t image_height,
		                VkFormat image_format = VK_FORMAT_R8G8B8A8_UNORM);
		void InitializeImageLayout(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer);

		ExternalHandle::ShareHandles ExportHandles();
		ExternalHandle::SharedImageInfo ExportImageInfo();

		void Cleanup();

	//private:
	public:
		VkDevice       device           {VK_NULL_HANDLE};
		VkImage        image            {VK_NULL_HANDLE};
		VkDeviceMemory memory           {VK_NULL_HANDLE};
		VkDeviceSize   size             {0};
		VkDeviceSize   allocationSize   {0};
		VkSampler      sampler          {VK_NULL_HANDLE};
		VkImageView    view             {VK_NULL_HANDLE};

		uint32_t image_width  = 0;
		uint32_t image_height = 0;
		VkFormat image_format = VK_FORMAT_R8G8B8A8_UNORM;

		struct SharedSemaphores
		{
			VkSemaphore ext_read  {VK_NULL_HANDLE};
			VkSemaphore ext_write {VK_NULL_HANDLE};
		};

		SharedSemaphores _shared_semaphores;
};

#endif //SHARED_IMAGE_VK_H
