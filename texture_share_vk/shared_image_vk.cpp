#include "texture_share_vk/shared_image_vk.h"

#include "texture_share_vk/logging.h"
#include "texture_share_vk/vk_helpers.h"

#include <functional>


SharedImageVk::SharedImageVk(VkDevice device)
    : device(device)
{}

SharedImageVk::SharedImageVk(SharedImageVk &&other)
{
	memcpy(this, &other, sizeof(SharedImageVk));
	other.device = VK_NULL_HANDLE;
}

SharedImageVk &SharedImageVk::operator=(SharedImageVk &&other)
{
	memcpy(this, &other, sizeof(SharedImageVk));
	other.device = VK_NULL_HANDLE;

	return *this;
}

SharedImageVk::~SharedImageVk()
{
	this->Cleanup();
}

void SharedImageVk::Initialize(VkDevice device, VkPhysicalDevice physical_device,
                               uint32_t image_width, uint32_t image_height,
                               VkFormat image_format)
{
	this->device = device;

	// Create semaphores. Ensure ExternalHandleVk::FindCompatibleSemaphoreProps() was already run before
	this->_shared_semaphores.ext_read  = ExternalHandleVk::CreateExternalSemaphore(device);
	this->_shared_semaphores.ext_write = ExternalHandleVk::CreateExternalSemaphore(device);

	// Allocate image memory
	VkExternalMemoryImageCreateInfo external_memory_image_create_info{VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
	external_memory_image_create_info.handleTypes = ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE;
	VkImageCreateInfo imageCreateInfo{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
	imageCreateInfo.pNext         = &external_memory_image_create_info;
	imageCreateInfo.imageType     = VK_IMAGE_TYPE_2D;
	imageCreateInfo.format        = image_format;
	imageCreateInfo.mipLevels     = 1;
	imageCreateInfo.arrayLayers   = 1;
	imageCreateInfo.samples       = VK_SAMPLE_COUNT_1_BIT;
	imageCreateInfo.extent.depth  = 1;
	imageCreateInfo.extent.width  = image_width;
	imageCreateInfo.extent.height = image_height;
	imageCreateInfo.usage         = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT |
	        VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
	VK_CHECK(vkCreateImage(device, &imageCreateInfo, nullptr, &this->image));

	this->image_height = image_height;
	this->image_width = image_width;
	this->image_format = image_format;

	VkMemoryRequirements memReqs{};
	vkGetImageMemoryRequirements(device, this->image, &memReqs);

	VkExportMemoryAllocateInfo exportAllocInfo{
		VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO, nullptr,
		ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE};
	VkMemoryAllocateInfo memAllocInfo{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, &exportAllocInfo};

	memAllocInfo.allocationSize = this->allocationSize = memReqs.size;
	memAllocInfo.memoryTypeIndex                               = VkHelpers::GetMemoryType(physical_device, memReqs.memoryTypeBits,
	                                                       VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->memory));
	VK_CHECK(vkBindImageMemory(device, this->image, this->memory, 0));

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

void SharedImageVk::InitializeImageLayout(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer)
{
	VkHelpers::ImmediateSubmit(device, queue, command_buffer,
	            [&](VkCommandBuffer image_command_buffer) {
		VkImageMemoryBarrier image_memory_barrier  = VkHelpers::CreateImageMemoryBarrier();
		image_memory_barrier.image                 = this->image;
		image_memory_barrier.srcAccessMask         = 0;
		image_memory_barrier.dstAccessMask         = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
		image_memory_barrier.oldLayout             = VK_IMAGE_LAYOUT_UNDEFINED;
		image_memory_barrier.newLayout             = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL; //VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
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

ExternalHandle::ShareHandles SharedImageVk::ExportHandles()
{
	ExternalHandle::ShareHandles handles;
	const auto memoryFdInfo = ExternalHandleVk::CreateMemoryGetInfoKHR(this->memory);
	ExternalHandleVk::GetMemoryKHR(device, &memoryFdInfo, &handles.memory);

	handles.ext_read = ExternalHandleVk::GetSemaphoreKHR(this->device, this->_shared_semaphores.ext_read);
	handles.ext_write = ExternalHandleVk::GetSemaphoreKHR(this->device, this->_shared_semaphores.ext_write);

	return handles;
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
