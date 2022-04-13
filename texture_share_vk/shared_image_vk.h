#ifndef SHARED_IMAGE_VK_H
#define SHARED_IMAGE_VK_H

#include "texture_share_vk/platform/platform.h"

#include <vulkan/vulkan.hpp>

class SharedImageVk
{
	public:
		static constexpr uint64_t DEFAULT_FENCE_TIMEOUT = 100000000000;

		SharedImageVk(VkDevice device);
		~SharedImageVk();

		void Initialize(VkDevice device, VkPhysicalDevice physical_device, uint32_t image_width, uint32_t image_height);
		void InitializeImageLayout(VkDevice device, VkQueue queue, VkCommandPool command_pool, VkCommandBuffer command_buffer);

		inline const ExternalHandleVk::TYPE &ReadSemaphoreHandle() const
		{	return this->_share_handles.ext_read;	}

		inline const ExternalHandleVk::TYPE &WriteSemaphoreHandle() const
		{	return this->_share_handles.ext_write;	}

		inline const ExternalHandleVk::TYPE &ImageMemoryHandle() const
		{	return this->_share_handles.memory;	}

		void Cleanup();

	private:
		VkDevice       device           {VK_NULL_HANDLE};
		VkImage        image            {VK_NULL_HANDLE};
		VkDeviceMemory memory           {VK_NULL_HANDLE};
		VkDeviceSize   size             {0};
		VkDeviceSize   allocationSize   {0};
		VkSampler      sampler          {VK_NULL_HANDLE};
		VkImageView    view             {VK_NULL_HANDLE};

		struct SharedSemaphores
		{
			VkSemaphore ext_read  {VK_NULL_HANDLE};
			VkSemaphore ext_write {VK_NULL_HANDLE};
		};

		SharedSemaphores _shared_semaphores;

		ExternalHandleVk::ShareHandles _share_handles;
};

#endif //SHARED_IMAGE_VK_H
