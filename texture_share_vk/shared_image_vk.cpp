#include "texture_share_vk/shared_image_vk.h"

#include "texture_share_vk/logging.h"

#include <functional>


SharedImageVk::SharedImageVk(VkDevice device)
    : device(device)
{}

SharedImageVk::~SharedImageVk()
{
	this->Cleanup();
}

uint32_t get_memory_type(VkPhysicalDevice physical_device, uint32_t bits, VkMemoryPropertyFlags properties, VkBool32 *memory_type_found = nullptr)
{
	VkPhysicalDeviceMemoryProperties memory_properties;
	vkGetPhysicalDeviceMemoryProperties(physical_device, &memory_properties);

	for (uint32_t i = 0; i < memory_properties.memoryTypeCount; i++)
	{
		if ((bits & 1) == 1)
		{
			if ((memory_properties.memoryTypes[i].propertyFlags & properties) == properties)
			{
				if (memory_type_found)
				{
					*memory_type_found = true;
				}
				return i;
			}
		}
		bits >>= 1;
	}

	if (memory_type_found)
	{
		*memory_type_found = false;
		return 0;
	}
	else
	{
		throw std::runtime_error("Could not find a matching memory type");
	}
}

/** @brief Initialize an image memory barrier with no image transfer ownership */
inline VkImageMemoryBarrier create_image_memory_barrier()
{
	VkImageMemoryBarrier image_memory_barrier{};
	image_memory_barrier.sType               = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
	image_memory_barrier.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
	image_memory_barrier.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
	return image_memory_barrier;
}


void immediate_submit(VkDevice device, VkQueue queue, VkCommandPool command_pool, VkCommandBuffer command_buffer, const std::function<void(VkCommandBuffer command_buffer)> &f, VkSemaphore signalSemaphore)
{
	f(command_buffer);

	if (command_buffer == VK_NULL_HANDLE)
		return;

	VK_CHECK(vkEndCommandBuffer(command_buffer));

	VkSubmitInfo submit_info{};
	submit_info.sType              = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit_info.commandBufferCount = 1;
	submit_info.pCommandBuffers    = &command_buffer;
	if (signalSemaphore)
	{
		submit_info.pSignalSemaphores    = &signalSemaphore;
		submit_info.signalSemaphoreCount = 1;
	}

	// Create fence to ensure that the command buffer has finished executing
	VkFenceCreateInfo fence_info{};
	fence_info.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
	fence_info.flags = 0;

	VkFence fence;
	VK_CHECK(vkCreateFence(device, &fence_info, nullptr, &fence));

	// Submit to the queue
	VkResult result = vkQueueSubmit(queue, 1, &submit_info, fence);
	// Wait for the fence to signal that command buffer has finished executing
	VK_CHECK(vkWaitForFences(device, 1, &fence, VK_TRUE, SharedImageVk::DEFAULT_FENCE_TIMEOUT));

	vkDestroyFence(device, fence, nullptr);

	if (command_pool)
		vkFreeCommandBuffers(device, command_pool, 1, &command_buffer);
}

void SharedImageVk::Initialize(VkDevice device, VkPhysicalDevice physical_device, uint32_t image_width, uint32_t image_height)
{
	// Create semaphores. Ensure FindCompatibleSemaphoreProps() was already run before
	VK_CHECK(vkCreateSemaphore(device, &ExternalHandleVk::ExternalSemaphoreCreateInfo(), nullptr,
	                           &this->_shared_semaphores.ext_read));
	VK_CHECK(vkCreateSemaphore(device, &ExternalHandleVk::ExternalSemaphoreCreateInfo(), nullptr,
	                           &this->_shared_semaphores.ext_write));

	// Allocate image memory
	VkExternalMemoryImageCreateInfo external_memory_image_create_info{VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
	external_memory_image_create_info.handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT_KHR;
	VkImageCreateInfo imageCreateInfo{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
	imageCreateInfo.pNext         = &external_memory_image_create_info;
	imageCreateInfo.imageType     = VK_IMAGE_TYPE_2D;
	imageCreateInfo.format        = VK_FORMAT_R8G8B8A8_UNORM;
	imageCreateInfo.mipLevels     = 1;
	imageCreateInfo.arrayLayers   = 1;
	imageCreateInfo.samples       = VK_SAMPLE_COUNT_1_BIT;
	imageCreateInfo.extent.depth  = 1;
	imageCreateInfo.extent.width  = image_width;
	imageCreateInfo.extent.height = image_height;
	imageCreateInfo.usage         = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT;
	VK_CHECK(vkCreateImage(device, &imageCreateInfo, nullptr, &this->image));

	VkMemoryRequirements memReqs{};
	vkGetImageMemoryRequirements(device, this->image, &memReqs);

	VkExportMemoryAllocateInfo exportAllocInfo{
		VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO, nullptr,
		ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE};
	VkMemoryAllocateInfo memAllocInfo{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, &exportAllocInfo};

	memAllocInfo.allocationSize = this->allocationSize = memReqs.size;
	memAllocInfo.memoryTypeIndex                               = get_memory_type(physical_device, memReqs.memoryTypeBits,
	                                                       VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->memory));
	VK_CHECK(vkBindImageMemory(device, this->image, this->memory, 0));

	VkMemoryGetFdInfoKHR memoryFdInfo = ExternalHandleVk::CreateMemoryGetInfoKHR(this->memory);
	ExternalHandleVk::GetMemoryKHR(device, &memoryFdInfo, &this->_share_handles.memory);

	// Create sampler
	VkSamplerCreateInfo samplerCreateInfo{VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO};
	samplerCreateInfo.magFilter  = VK_FILTER_LINEAR;
	samplerCreateInfo.minFilter  = VK_FILTER_LINEAR;
	samplerCreateInfo.mipmapMode = VK_SAMPLER_MIPMAP_MODE_LINEAR;
	samplerCreateInfo.maxLod     = (float) 1;
	//samplerCreateInfo.maxAnisotropy = context.deviceFeatures.samplerAnisotropy ? context.deviceProperties.limits.maxSamplerAnisotropy : 1.0f;
	//samplerCreateInfo.anisotropyEnable = context.deviceFeatures.samplerAnisotropy;
	samplerCreateInfo.borderColor = VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE;
	vkCreateSampler(device, &samplerCreateInfo, nullptr, &this->sampler);

	// Create image view
	VkImageViewCreateInfo viewCreateInfo{VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO};
	viewCreateInfo.viewType         = VK_IMAGE_VIEW_TYPE_2D;
	viewCreateInfo.image            = this->image;
	viewCreateInfo.format           = VK_FORMAT_R8G8B8A8_UNORM;
	viewCreateInfo.subresourceRange = VkImageSubresourceRange{VK_IMAGE_ASPECT_COLOR_BIT, 0, 1,
	                                                          0, 1};
	vkCreateImageView(device, &viewCreateInfo, nullptr, &this->view);
}

void SharedImageVk::InitializeImageLayout(VkDevice device, VkQueue queue, VkCommandPool command_pool, VkCommandBuffer command_buffer)
{
	immediate_submit(device, queue, command_pool, command_buffer,
	            [&](VkCommandBuffer image_command_buffer) {
		VkImageMemoryBarrier image_memory_barrier  = create_image_memory_barrier();
		image_memory_barrier.image                 = this->image;
		image_memory_barrier.srcAccessMask         = 0;
		image_memory_barrier.dstAccessMask         = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
		image_memory_barrier.oldLayout             = VK_IMAGE_LAYOUT_UNDEFINED;
		image_memory_barrier.newLayout             = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
		VkImageSubresourceRange &subresource_range = image_memory_barrier.subresourceRange;
		subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		subresource_range.levelCount               = 1;
		subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(
		    image_command_buffer,
		    VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
		    VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
		    0,
		    0, nullptr,
		    0, nullptr,
		    1, &image_memory_barrier);
	},
	this->_shared_semaphores.ext_write);
}

void SharedImageVk::Cleanup()
{
	if(this->device)
	{
		vkDeviceWaitIdle(this->device);
		vkDestroySemaphore(this->device, this->_shared_semaphores.ext_read, nullptr);
		vkDestroySemaphore(this->device, this->_shared_semaphores.ext_write, nullptr);
		vkDestroyImage(this->device, this->image, nullptr);
		vkDestroySampler(this->device, this->sampler, nullptr);
		vkDestroyImageView(this->device, this->view, nullptr);
		vkFreeMemory(this->device, this->memory, nullptr);

		this->device = {VK_NULL_HANDLE};
	}
}
