#include "vk_shared_image.h"

#include "platform/external_handle_vk.h"
#include "vk_shared_image/platform/linux/external_handle_vk.h"
#include "vk_shared_image/vk_helpers.h"
#include <memory>
#include <vulkan/vulkan_core.h>

VkSharedImage::~VkSharedImage()
{
	this->Cleanup();
}

void VkSharedImage::InitializeVulkan(VkInstance instance, VkPhysicalDevice physical_device)
{
	ExternalHandleVk::LoadVulkanHandleExtensions(instance);
	ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(physical_device);
}

void VkSharedImage::Initialize(VkDevice device, VkPhysicalDevice physical_device, VkQueue queue,
                               VkCommandBuffer command_buffer, uint32_t width, uint32_t height, VkFormat format,
                               uint32_t id)
{
	this->Cleanup();

	this->_device = device;

	// Create semaphores. Ensure ExternalHandleVk::FindCompatibleSemaphoreProps() was already run before
	// this->_shared_semaphores.ext_read  = ExternalHandleVk::CreateExternalSemaphore(device);
	// this->_shared_semaphores.ext_write = ExternalHandleVk::CreateExternalSemaphore(device);

	// Allocate image memory
	VkExternalMemoryImageCreateInfo external_memory_image_create_info{
		VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
	external_memory_image_create_info.handleTypes = ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE;
	VkImageCreateInfo imageCreateInfo{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
	imageCreateInfo.pNext         = &external_memory_image_create_info;
	imageCreateInfo.imageType     = VK_IMAGE_TYPE_2D;
	imageCreateInfo.format        = format;
	imageCreateInfo.mipLevels     = 1;
	imageCreateInfo.arrayLayers   = 1;
	imageCreateInfo.samples       = VK_SAMPLE_COUNT_1_BIT;
	imageCreateInfo.extent.depth  = 1;
	imageCreateInfo.extent.width  = width;
	imageCreateInfo.extent.height = height;
	imageCreateInfo.usage         = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT |
	                        VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
	VK_CHECK(vkCreateImage(device, &imageCreateInfo, nullptr, &this->_image));

	this->_data.Height = height;
	this->_data.Width  = width;
	this->_data.Format = format;
	this->_data.Id     = id;

	VkMemoryRequirements memReqs{};
	vkGetImageMemoryRequirements(device, this->_image, &memReqs);

	VkExportMemoryAllocateInfo exportAllocInfo{VK_STRUCTURE_TYPE_EXPORT_MEMORY_ALLOCATE_INFO, nullptr,
	                                           ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE};
	VkMemoryAllocateInfo memAllocInfo{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, &exportAllocInfo};

	memAllocInfo.allocationSize = this->_data.AllocationSize = memReqs.size;
	memAllocInfo.memoryTypeIndex =
		VkHelpers::GetMemoryType(physical_device, memReqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->_memory));
	VK_CHECK(vkBindImageMemory(device, this->_image, this->_memory, 0));

	// // Create sampler
	// VkSamplerCreateInfo samplerCreateInfo{VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO};
	// samplerCreateInfo.magFilter  = VK_FILTER_LINEAR;
	// samplerCreateInfo.minFilter  = VK_FILTER_LINEAR;
	// samplerCreateInfo.mipmapMode = VK_SAMPLER_MIPMAP_MODE_LINEAR;
	// samplerCreateInfo.maxLod     = (float) 1;
	// samplerCreateInfo.borderColor = VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE;
	// vkCreateSampler(device, &samplerCreateInfo, nullptr, &this->sampler);

	// // Create image view
	// VkImageViewCreateInfo viewCreateInfo{VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO};
	// viewCreateInfo.viewType         = VK_IMAGE_VIEW_TYPE_2D;
	// viewCreateInfo.image            = this->_image;
	// viewCreateInfo.format           = format;
	// viewCreateInfo.subresourceRange = VkImageSubresourceRange{VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
	// vkCreateImageView(device, &viewCreateInfo, nullptr, &this->_view);

	// Initialize image
	VkFence fence                  = VK_NULL_HANDLE;
	VkFenceCreateInfo fence_create = {VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, nullptr, 0};
	vkCreateFence(device, &fence_create, nullptr, &fence);

	this->SetImageLayout(queue, command_buffer, VK_IMAGE_LAYOUT_GENERAL, fence);

	vkDestroyFence(device, fence, nullptr);
}

void VkSharedImage::Cleanup()
{
	if(this->_device)
	{
		vkDeviceWaitIdle(this->_device);

		// vkDestroySemaphore(this->_device, this->_shared_semaphores.ext_read, nullptr);
		// vkDestroySemaphore(this->_device, this->_shared_semaphores.ext_write, nullptr);
		vkDestroyImage(this->_device, this->_image, nullptr);
		// vkDestroySampler(this->_device, this->sampler, nullptr);
		// vkDestroyImageView(this->_device, this->_view, nullptr);
		vkFreeMemory(this->_device, this->_memory, nullptr);

		this->_device = VK_NULL_HANDLE;
	}
}

void VkSharedImage::ImportFromHandle(VkDevice device, VkPhysicalDevice physical_device,
                                     ExternalHandle::ShareHandles &&share_handles, const SharedImageData &image_data)
{
	this->Cleanup();

	this->_device = device;

	// Import Semaphores
	// this->_semaphore_read  = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_read);
	// this->_semaphore_write = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_write);

	// Create and allocate image memory
	this->_data = image_data;

	VkExternalMemoryImageCreateInfo external_memory_image_create_info{
		VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
	external_memory_image_create_info.handleTypes = ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE;
	VkImageCreateInfo imageCreateInfo{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
	imageCreateInfo.pNext         = &external_memory_image_create_info;
	imageCreateInfo.imageType     = VK_IMAGE_TYPE_2D;
	imageCreateInfo.format        = this->_data.Format;
	imageCreateInfo.mipLevels     = 1;
	imageCreateInfo.arrayLayers   = 1;
	imageCreateInfo.samples       = VK_SAMPLE_COUNT_1_BIT;
	imageCreateInfo.extent.depth  = 1;
	imageCreateInfo.extent.width  = this->_data.Width;
	imageCreateInfo.extent.height = this->_data.Height;
	imageCreateInfo.usage         = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT |
	                        VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
	VK_CHECK(vkCreateImage(device, &imageCreateInfo, nullptr, &this->_image));

	VkMemoryRequirements memReqs{};
	vkGetImageMemoryRequirements(device, this->_image, &memReqs);

	const auto import_memory_info = ExternalHandleVk::CreateImportMemoryInfoKHR(share_handles.memory);

	VkMemoryAllocateInfo memAllocInfo{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, &import_memory_info};
	memAllocInfo.allocationSize = memReqs.size;
	memAllocInfo.memoryTypeIndex =
		VkHelpers::GetMemoryType(physical_device, memReqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->_memory));

	VK_CHECK(vkBindImageMemory(device, this->_image, this->_memory, 0));

	// File descriptor ownership transferred to vulkan. Prevent clos on destructor call
	share_handles.memory = ExternalHandle::INVALID_VALUE;
	// external_handles.handles.ext_read  = ExternalHandle::INVALID_VALUE;
	// external_handles.handles.ext_write = ExternalHandle::INVALID_VALUE;
}

VkImageSubresourceLayers VkSharedImage::CreateColorSubresourceLayer()
{
	VkImageSubresourceLayers layer{};
	layer.aspectMask     = VK_IMAGE_ASPECT_COLOR_BIT;
	layer.baseArrayLayer = 0;
	layer.layerCount     = 1;
	layer.mipLevel       = 0;

	return layer;
}

void VkSharedImage::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
                                  VkImageLayout dst_image_layout, VkFence fence, const VkOffset3D dst_image_extent[2])
{
	const VkOffset3D src_image_extent[2] = {
		{0,									   0,										0},
		{static_cast<int32_t>(this->_data.Width), static_cast<int32_t>(this->_data.Height), 1}
    };

	return this->ImageBlit(graphics_queue, command_buffer, this->_image, this->_layout, src_image_extent, dst_image,
	                       dst_image_layout, dst_image_extent, fence);
}

void VkSharedImage::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
                                  VkImageLayout dst_image_layout, VkFence fence)
{
	const VkOffset3D dst_image_extent[2] = {
		{0,									   0,										0},
		{static_cast<int32_t>(this->_data.Width), static_cast<int32_t>(this->_data.Height), 1}
    };

	return this->SendImageBlit(graphics_queue, command_buffer, dst_image, dst_image_layout, fence, dst_image_extent);
}

void VkSharedImage::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
                                  VkImageLayout src_image_layout, VkFence fence, const VkOffset3D src_image_extent[2])
{
	const VkOffset3D dst_image_extent[2] = {
		{0,									   0,										0},
		{static_cast<int32_t>(this->_data.Width), static_cast<int32_t>(this->_data.Height), 1}
    };

	return this->ImageBlit(graphics_queue, command_buffer, src_image, src_image_layout, src_image_extent, this->_image,
	                       this->_layout, dst_image_extent, fence);
}

void VkSharedImage::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
                                  VkImageLayout src_image_layout, VkFence fence)
{
	const VkOffset3D src_image_extent[2] = {
		{0,									   0,										0},
		{static_cast<int32_t>(this->_data.Width), static_cast<int32_t>(this->_data.Height), 1}
    };

	return this->RecvImageBlit(graphics_queue, command_buffer, src_image, src_image_layout, fence, src_image_extent);
}

ExternalHandle::ShareHandles VkSharedImage::ExportHandles()
{
	ExternalHandle::ShareHandles handles;
	const auto memoryFdInfo = ExternalHandleVk::CreateMemoryGetInfoKHR(this->_memory);
	ExternalHandleVk::GetMemoryKHR(this->_device, &memoryFdInfo, &handles.memory);

	// handles.ext_read = ExternalHandleVk::GetSemaphoreKHR(this->_device, this->_shared_semaphores.ext_read);
	// handles.ext_write = ExternalHandleVk::GetSemaphoreKHR(this->_device, this->_shared_semaphores.ext_write);

	return handles;
}

void VkSharedImage::SetImageLayout(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImageLayout target_layout,
                                   VkFence fence)
{
	// naming it cmd for shorter writing
	VkCommandBuffer cmd                     = command_buffer;
	VkCommandBufferBeginInfo cmd_begin_info = VkHelpers::CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	VkImageMemoryBarrier mem_barrier = {VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};

	// Image memory barrier before entering VK_PIPELINE_STAGE_TRANSFER_BIT
	mem_barrier.image               = this->_image;
	mem_barrier.srcAccessMask       = VK_ACCESS_NONE;
	mem_barrier.dstAccessMask       = VK_ACCESS_TRANSFER_READ_BIT;
	mem_barrier.oldLayout           = this->_layout;
	mem_barrier.newLayout           = target_layout;
	mem_barrier.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
	mem_barrier.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;

	VkImageSubresourceRange &src_img_subresource_range = mem_barrier.subresourceRange;
	src_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
	src_img_subresource_range.levelCount               = 1;
	src_img_subresource_range.layerCount               = 1;

	vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0, 0,
	                     nullptr, 0, nullptr, 1, &mem_barrier);

	VK_CHECK(vkEndCommandBuffer(cmd));

	// prepare the submission to the queue.
	// we want to wait on the _presentSemaphore, as that semaphore is signaled when the swapchain is ready
	// we will signal the _renderSemaphore, to signal that rendering has finished

	VkSubmitInfo submit = {};
	submit.sType        = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit.pNext        = nullptr;

	// VkPipelineStageFlags wait_stage[] = {VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT};
	// submit.pWaitDstStageMask          = wait_stage;

	//	VkSemaphore wait_semaphores[] = {this->_semaphore_read, this->_semaphore_write};
	//	submit.waitSemaphoreCount     = 2;
	//	submit.pWaitSemaphores        = wait_semaphores;

	//	VkSemaphore signal_semaphores[] = {this->_semaphore_write};
	//	submit.signalSemaphoreCount     = submit.waitSemaphoreCount;
	//	submit.pSignalSemaphores        = submit.pWaitSemaphores;

	submit.commandBufferCount = 1;
	submit.pCommandBuffers    = &command_buffer;

	// submit command buffer to the queue and execute it.
	//  if set, fence may block until the graphic commands finish execution
	VK_CHECK(vkQueueSubmit(graphics_queue, 1, &submit, fence));

	this->_layout = target_layout;

	// Wait for the fence to signal that command buffer has finished executing
	if(fence != VK_NULL_HANDLE)
	{
		VK_CHECK(vkWaitForFences(this->_device, 1, &fence, VK_TRUE, VkHelpers::DEFAULT_FENCE_TIMEOUT));
		VK_CHECK(vkResetFences(this->_device, 1, &fence));
	}
}

void VkSharedImage::ImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
                              VkImageLayout src_image_layout, const VkOffset3D src_image_extent[2], VkImage dst_image,
                              VkImageLayout dst_image_layout, const VkOffset3D dst_image_extent[2], VkFence fence)
{
	// naming it cmd for shorter writing
	VkCommandBuffer cmd                     = command_buffer;
	VkCommandBufferBeginInfo cmd_begin_info = VkHelpers::CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	VkImageMemoryBarrier mem_barriers[2] = {{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER},
	                                        {VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER}};

	constexpr VkImageLayout src_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
	constexpr VkImageLayout dst_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;

	// Image memory barrier before entering VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &src_img_mem_barrier = mem_barriers[0];
		src_img_mem_barrier.image                 = src_image;
		src_img_mem_barrier.srcAccessMask         = VK_ACCESS_NONE;
		src_img_mem_barrier.dstAccessMask         = VK_ACCESS_TRANSFER_READ_BIT;
		src_img_mem_barrier.oldLayout             = src_image_layout;
		src_img_mem_barrier.newLayout             = src_requested_layout;
		src_img_mem_barrier.srcQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;
		src_img_mem_barrier.dstQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;

		VkImageSubresourceRange &src_img_subresource_range = src_img_mem_barrier.subresourceRange;
		src_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		src_img_subresource_range.levelCount               = 1;
		src_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &dst_img_mem_barrier = mem_barriers[1];
		dst_img_mem_barrier.image                 = dst_image;
		dst_img_mem_barrier.srcAccessMask         = VK_ACCESS_NONE;
		dst_img_mem_barrier.dstAccessMask         = VK_ACCESS_TRANSFER_WRITE_BIT;
		dst_img_mem_barrier.oldLayout             = dst_image_layout;
		dst_img_mem_barrier.newLayout             = dst_requested_layout;
		dst_img_mem_barrier.srcQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;
		dst_img_mem_barrier.dstQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;

		VkImageSubresourceRange &dst_img_subresource_range = dst_img_mem_barrier.subresourceRange;
		dst_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		dst_img_subresource_range.levelCount               = 1;
		dst_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0,
		                     nullptr, 0, nullptr, 2, mem_barriers);
	}

	VkImageBlit region{};
	region.srcSubresource = CreateColorSubresourceLayer();
	region.dstSubresource = CreateColorSubresourceLayer();

	memcpy(&region.srcOffsets, src_image_extent, 2 * sizeof(VkOffset3D));
	memcpy(&region.dstOffsets, dst_image_extent, 2 * sizeof(VkOffset3D));

	vkCmdBlitImage(command_buffer, src_image, src_requested_layout, dst_image, dst_requested_layout, 1, &region,
	               VK_FILTER_NEAREST);

	// Image memory barrier after exiting VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &src_mem_barrier              = mem_barriers[0];
		src_mem_barrier.image                              = src_image;
		src_mem_barrier.srcAccessMask                      = VK_ACCESS_TRANSFER_READ_BIT;
		src_mem_barrier.dstAccessMask                      = VK_ACCESS_NONE;
		src_mem_barrier.oldLayout                          = src_requested_layout;
		src_mem_barrier.newLayout                          = src_image_layout;
		src_mem_barrier.srcQueueFamilyIndex                = VK_QUEUE_FAMILY_IGNORED;
		src_mem_barrier.dstQueueFamilyIndex                = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &src_img_subresource_range = src_mem_barrier.subresourceRange;
		src_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		src_img_subresource_range.levelCount               = 1;
		src_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &dst_mem_barrier              = mem_barriers[1];
		dst_mem_barrier.image                              = dst_image;
		dst_mem_barrier.srcAccessMask                      = VK_ACCESS_TRANSFER_WRITE_BIT;
		dst_mem_barrier.dstAccessMask                      = VK_ACCESS_NONE;
		dst_mem_barrier.oldLayout                          = dst_requested_layout;
		dst_mem_barrier.newLayout                          = dst_image_layout;
		dst_mem_barrier.srcQueueFamilyIndex                = VK_QUEUE_FAMILY_IGNORED;
		dst_mem_barrier.dstQueueFamilyIndex                = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &dst_img_subresource_range = dst_mem_barrier.subresourceRange;
		dst_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		dst_img_subresource_range.levelCount               = 1;
		dst_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0, 0,
		                     nullptr, 0, nullptr, 2, mem_barriers);
	}

	VK_CHECK(vkEndCommandBuffer(cmd));

	// prepare the submission to the queue.
	// we want to wait on the _presentSemaphore, as that semaphore is signaled when the swapchain is ready
	// we will signal the _renderSemaphore, to signal that rendering has finished

	VkSubmitInfo submit = {};
	submit.sType        = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit.pNext        = nullptr;

	// VkPipelineStageFlags wait_stage[] = {VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT};
	// submit.pWaitDstStageMask          = wait_stage;

	//	VkSemaphore wait_semaphores[] = {this->_semaphore_read, this->_semaphore_write};
	//	submit.waitSemaphoreCount     = 2;
	//	submit.pWaitSemaphores        = wait_semaphores;

	//	VkSemaphore signal_semaphores[] = {this->_semaphore_write};
	//	submit.signalSemaphoreCount     = submit.waitSemaphoreCount;
	//	submit.pSignalSemaphores        = submit.pWaitSemaphores;

	submit.commandBufferCount = 1;
	submit.pCommandBuffers    = &command_buffer;

	// submit command buffer to the queue and execute it.
	//  if set, fence may block until the graphic commands finish execution
	VK_CHECK(vkQueueSubmit(graphics_queue, 1, &submit, fence));

	// Wait for the fence to signal that command buffer has finished executing
	if(fence != VK_NULL_HANDLE)
	{
		VK_CHECK(vkWaitForFences(this->_device, 1, &fence, VK_TRUE, VkHelpers::DEFAULT_FENCE_TIMEOUT));
		VK_CHECK(vkResetFences(this->_device, 1, &fence));
	}
}
