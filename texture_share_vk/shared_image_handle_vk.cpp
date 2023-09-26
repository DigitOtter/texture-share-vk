#include "texture_share_vk/shared_image_handle_vk.h"

#include "texture_share_vk/vk_helpers.h"

SharedImageHandleVk::~SharedImageHandleVk()
{
	this->Cleanup();
}

SharedImageHandleVk &SharedImageHandleVk::operator=(SharedImageHandleVk &&other)
{
	this->~SharedImageHandleVk();

	this->_format = std::move(other._format);
	this->_height = std::move(other._height);
	this->_width  = std::move(other._width);
	this->_handle_id = std::move(other._handle_id);

	this->_image_layout = std::move(other._image_layout);

	this->_semaphore_write = std::move(other._semaphore_write);
	other._semaphore_write = VK_NULL_HANDLE;

	this->_semaphore_read = std::move(other._semaphore_read);
	other._semaphore_read = VK_NULL_HANDLE;

	this->_image_memory = std::move(other._image_memory);
	other._image_memory = VK_NULL_HANDLE;

	this->_image = std::move(other._image);
	other._image = VK_NULL_HANDLE;

	this->_device = std::move(other._device);
	other._device = VK_NULL_HANDLE;

	return *this;
}

void SharedImageHandleVk::ImportHandles(VkDevice device, VkPhysicalDevice physical_device,
                                        ExternalHandle::SharedImageInfo &&external_handles)
{
	this->_device = device;

	// Import Semaphores
	this->_semaphore_read  = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_read);
	this->_semaphore_write = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_write);

	// Create and allocate image memory
	this->_width  = external_handles.width;
	this->_height = external_handles.height;
	this->_format = ExternalHandleVk::GetVkFormat(external_handles.format);
	this->_handle_id = external_handles.handle_id;

	VkExternalMemoryImageCreateInfo external_memory_image_create_info{
		VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
	external_memory_image_create_info.handleTypes = ExternalHandleVk::EXTERNAL_MEMORY_HANDLE_TYPE;
	VkImageCreateInfo imageCreateInfo{VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO};
	imageCreateInfo.pNext         = &external_memory_image_create_info;
	imageCreateInfo.imageType     = VK_IMAGE_TYPE_2D;
	imageCreateInfo.format        = this->_format;
	imageCreateInfo.mipLevels     = 1;
	imageCreateInfo.arrayLayers   = 1;
	imageCreateInfo.samples       = VK_SAMPLE_COUNT_1_BIT;
	imageCreateInfo.extent.depth  = 1;
	imageCreateInfo.extent.width  = this->_width;
	imageCreateInfo.extent.height = this->_height;
	imageCreateInfo.usage         = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT |
	                        VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
	VK_CHECK(vkCreateImage(device, &imageCreateInfo, nullptr, &this->_image));

	VkMemoryRequirements memReqs{};
	vkGetImageMemoryRequirements(device, this->_image, &memReqs);

	const auto import_memory_info = ExternalHandleVk::CreateImportMemoryInfoKHR(external_handles.handles.memory);

	VkMemoryAllocateInfo memAllocInfo{VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO, &import_memory_info};
	memAllocInfo.allocationSize = memReqs.size;
	memAllocInfo.memoryTypeIndex =
		VkHelpers::GetMemoryType(physical_device, memReqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->_image_memory));

	VK_CHECK(vkBindImageMemory(device, this->_image, this->_image_memory, 0));

	// File descriptor ownership transferred to vulkan. Prevent clos on destructor call
	external_handles.handles.memory    = ExternalHandle::INVALID_VALUE;
	external_handles.handles.ext_read  = ExternalHandle::INVALID_VALUE;
	external_handles.handles.ext_write = ExternalHandle::INVALID_VALUE;
}

void SharedImageHandleVk::SetImageLayout(VkQueue graphics_queue, VkCommandBuffer command_buffer,
                                         VkImageLayout image_layout)
{
	VkHelpers::ImmediateSubmit(
		this->_device, graphics_queue, command_buffer,
		[&](VkCommandBuffer image_command_buffer) {
			VkHelpers::CmdPipelineMemoryBarrierColorImage(image_command_buffer, this->_image, this->_image_layout,
		                                                  image_layout, VK_ACCESS_NONE,
		                                                  VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
		},
		this->_semaphore_write);

	this->_image_layout = image_layout;
}

void SharedImageHandleVk::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image,
                                        VkImageLayout send_image_layout, VkFence fence)
{
	const VkOffset3D srcOffset[2] = {
		{0,								  0,								   0},
		{static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
    };

	return this->SendImageBlit(graphics_queue, command_buffer, send_image, send_image_layout, fence, srcOffset);
}

void SharedImageHandleVk::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image,
                                        VkImageLayout send_image_layout, VkFence fence,
                                        const VkOffset3D send_image_extent[2])
{
	// naming it cmd for shorter writing
	VkCommandBuffer cmd = command_buffer;
	VkCommandBufferBeginInfo cmd_begin_info = VkHelpers::CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	VkImageMemoryBarrier mem_barriers[2] = {{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER},
	                                        {VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER}};

	constexpr VkImageLayout shared_image_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
	constexpr VkImageLayout target_image_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;

	// Image memory barrier before entering VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &shared_img_mem_barrier          = mem_barriers[0];
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		shared_img_mem_barrier.oldLayout                      = this->_image_layout;
		shared_img_mem_barrier.newLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &target_img_mem_barrier          = mem_barriers[1];
		target_img_mem_barrier.image                          = send_image;
		target_img_mem_barrier.srcAccessMask                  = VK_ACCESS_NONE;
		target_img_mem_barrier.dstAccessMask                  = VK_ACCESS_TRANSFER_READ_BIT;
		target_img_mem_barrier.oldLayout                      = send_image_layout;
		target_img_mem_barrier.newLayout                      = target_image_requested_layout;
		target_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		target_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &target_img_subresource_range = target_img_mem_barrier.subresourceRange;
		target_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		target_img_subresource_range.levelCount               = 1;
		target_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0,
		                     nullptr, 0, nullptr, 2, mem_barriers);
	}

	VkImageBlit region{};
	region.srcSubresource = CreateColorSubresourceLayer();
	region.dstSubresource = CreateColorSubresourceLayer();

	memcpy(&region.srcOffsets, send_image_extent, 2 * sizeof(VkOffset3D));

	region.dstOffsets[0].x = 0;
	region.dstOffsets[0].y = 0;
	region.dstOffsets[0].z = 0;

	region.dstOffsets[1].x = this->_width;
	region.dstOffsets[1].y = this->_height;
	region.dstOffsets[1].z = 1;

	vkCmdBlitImage(command_buffer, send_image, target_image_requested_layout, this->_image,
	               shared_image_requested_layout, 1, &region, VK_FILTER_NEAREST);

	// Image memory barrier after exiting VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &shared_img_mem_barrier          = mem_barriers[0];
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.oldLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.newLayout                      = this->_image_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &target_img_mem_barrier          = mem_barriers[1];
		target_img_mem_barrier.image                          = send_image;
		target_img_mem_barrier.srcAccessMask                  = VK_ACCESS_TRANSFER_READ_BIT;
		target_img_mem_barrier.dstAccessMask                  = VK_ACCESS_NONE;
		target_img_mem_barrier.oldLayout                      = target_image_requested_layout;
		target_img_mem_barrier.newLayout                      = send_image_layout;
		target_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		target_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &target_img_subresource_range = target_img_mem_barrier.subresourceRange;
		target_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		target_img_subresource_range.levelCount               = 1;
		target_img_subresource_range.layerCount               = 1;

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

void SharedImageHandleVk::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image,
                                        VkImageLayout pre_recv_image_layout, VkImageLayout post_recv_image_layout,
                                        VkFence fence)
{
	const VkOffset3D dstOffset[2] = {
		{0,								  0,								   0},
		{static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
    };

	return this->RecvImageBlit(graphics_queue, command_buffer, recv_image, pre_recv_image_layout,
	                           post_recv_image_layout, fence, dstOffset);
}

void SharedImageHandleVk::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image,
                                        VkImageLayout pre_recv_image_layout, VkImageLayout post_recv_image_layout,
                                        VkFence fence, const VkOffset3D recv_image_extent[2])
{
	// naming it cmd for shorter writing
	VkCommandBuffer cmd = command_buffer;
	VkCommandBufferBeginInfo cmd_begin_info = VkHelpers::CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	VkImageMemoryBarrier mem_barriers[2] = {{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER},
	                                        {VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER}};

	constexpr VkImageLayout shared_image_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
	constexpr VkImageLayout target_image_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;

	// Image memory barrier before entering VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &shared_img_mem_barrier          = mem_barriers[0];
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_TRANSFER_READ_BIT;
		shared_img_mem_barrier.oldLayout                      = this->_image_layout;
		shared_img_mem_barrier.newLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &target_img_mem_barrier          = mem_barriers[1];
		target_img_mem_barrier.image                          = recv_image;
		target_img_mem_barrier.srcAccessMask                  = VK_ACCESS_NONE;
		target_img_mem_barrier.dstAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		target_img_mem_barrier.oldLayout                      = pre_recv_image_layout;
		target_img_mem_barrier.newLayout                      = target_image_requested_layout;
		target_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		target_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &target_img_subresource_range = target_img_mem_barrier.subresourceRange;
		target_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		target_img_subresource_range.levelCount               = 1;
		target_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0,
		                     nullptr, 0, nullptr, 2, mem_barriers);
	}

	VkImageBlit region{};
	region.srcSubresource = CreateColorSubresourceLayer();
	region.dstSubresource = CreateColorSubresourceLayer();

	region.srcOffsets[0].x = 0;
	region.srcOffsets[0].y = 0;
	region.srcOffsets[0].z = 0;

	region.srcOffsets[1].x = this->_width;
	region.srcOffsets[1].y = this->_height;
	region.srcOffsets[1].z = 1;

	memcpy(&region.dstOffsets, recv_image_extent, 2 * sizeof(VkOffset3D));

	vkCmdBlitImage(command_buffer, this->_image, shared_image_requested_layout, recv_image,
	               target_image_requested_layout, 1, &region, VK_FILTER_NEAREST);

	// Image memory barrier after exiting VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier &shared_img_mem_barrier          = mem_barriers[0];
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_TRANSFER_READ_BIT;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.oldLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.newLayout                      = this->_image_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		VkImageMemoryBarrier &target_img_mem_barrier          = mem_barriers[1];
		target_img_mem_barrier.image                          = recv_image;
		target_img_mem_barrier.srcAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		target_img_mem_barrier.dstAccessMask                  = VK_ACCESS_NONE;
		target_img_mem_barrier.oldLayout                      = target_image_requested_layout;
		target_img_mem_barrier.newLayout                      = post_recv_image_layout;
		target_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		target_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &target_img_subresource_range = target_img_mem_barrier.subresourceRange;
		target_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		target_img_subresource_range.levelCount               = 1;
		target_img_subresource_range.layerCount               = 1;

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

	//	VkPipelineStageFlags wait_stage[] = {VK_PIPELINE_STAGE_TRANSFER_BIT};
	//	submit.pWaitDstStageMask          = wait_stage;

	//	VkSemaphore wait_semaphores[] = {this->_semaphore_write};
	//	submit.waitSemaphoreCount     = 1;
	//	submit.pWaitSemaphores        = wait_semaphores;

	//	VkSemaphore signal_semaphores[] = {this->_semaphore_read, this->_semaphore_write};
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

void SharedImageHandleVk::ClearImage(VkQueue graphics_queue, VkCommandBuffer command_buffer,
                                     VkClearColorValue clear_color, VkFence fence)
{
	// TODO: Test
	// naming it cmd for shorter writing
	VkCommandBuffer cmd = command_buffer;
	VkCommandBufferBeginInfo cmd_begin_info = VkHelpers::CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	constexpr VkImageLayout shared_image_requested_layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;

	// Image memory barrier before entering VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier shared_img_mem_barrier{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		shared_img_mem_barrier.oldLayout                      = this->_image_layout;
		shared_img_mem_barrier.newLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT, 0, 0,
		                     nullptr, 0, nullptr, 1, &shared_img_mem_barrier);
	}

	VkImageSubresourceRange img_range;
	img_range.aspectMask     = VK_IMAGE_ASPECT_COLOR_BIT;
	img_range.baseArrayLayer = 0;
	img_range.layerCount     = 1;
	img_range.baseMipLevel   = 0;
	img_range.levelCount     = 1;

	vkCmdClearColorImage(command_buffer, this->_image, shared_image_requested_layout, &clear_color, 1, &img_range);

	// Image memory barrier after exiting VK_PIPELINE_STAGE_TRANSFER_BIT
	{
		VkImageMemoryBarrier shared_img_mem_barrier{VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};
		shared_img_mem_barrier.image                          = this->_image;
		shared_img_mem_barrier.srcAccessMask                  = VK_ACCESS_TRANSFER_WRITE_BIT;
		shared_img_mem_barrier.dstAccessMask                  = VK_ACCESS_NONE;
		shared_img_mem_barrier.oldLayout                      = shared_image_requested_layout;
		shared_img_mem_barrier.newLayout                      = this->_image_layout;
		shared_img_mem_barrier.srcQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		shared_img_mem_barrier.dstQueueFamilyIndex            = VK_QUEUE_FAMILY_IGNORED;
		VkImageSubresourceRange &shared_img_subresource_range = shared_img_mem_barrier.subresourceRange;
		shared_img_subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
		shared_img_subresource_range.levelCount               = 1;
		shared_img_subresource_range.layerCount               = 1;

		vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT, 0, 0,
		                     nullptr, 0, nullptr, 1, &shared_img_mem_barrier);
	}

	VK_CHECK(vkEndCommandBuffer(cmd));

	// prepare the submission to the queue.
	// we want to wait on the _presentSemaphore, as that semaphore is signaled when the swapchain is ready
	// we will signal the _renderSemaphore, to signal that rendering has finished

	VkSubmitInfo submit = {};
	submit.sType        = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit.pNext        = nullptr;

	//	VkPipelineStageFlags wait_stage[] = {VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT};
	//	submit.pWaitDstStageMask          = wait_stage;

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

void SharedImageHandleVk::Cleanup()
{
	if(!this->_device)
		return;

	vkDeviceWaitIdle(this->_device);

	if(this->_image)
		vkDestroyImage(this->_device, this->_image, nullptr);

	if(this->_image_memory)
		vkFreeMemory(this->_device, this->_image_memory, nullptr);

	if(this->_semaphore_read)
		vkDestroySemaphore(this->_device, this->_semaphore_read, nullptr);
	if(this->_semaphore_write)
		vkDestroySemaphore(this->_device, this->_semaphore_write, nullptr);

	this->_device = VK_NULL_HANDLE;
}

VkImageSubresourceLayers SharedImageHandleVk::CreateColorSubresourceLayer()
{
	VkImageSubresourceLayers layer{};
	layer.aspectMask     = VK_IMAGE_ASPECT_COLOR_BIT;
	layer.baseArrayLayer = 0;
	layer.layerCount     = 1;
	layer.mipLevel       = 0;

	return layer;
}

VkSemaphore SharedImageHandleVk::ImportSemaphoreHandle(VkDevice device, ExternalHandle::TYPE semaphore_handle)
{
	return ExternalHandleVk::CreateImportSemaphoreKHR(device, semaphore_handle);
}
