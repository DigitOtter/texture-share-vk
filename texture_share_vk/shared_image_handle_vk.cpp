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
	this->_width = std::move(other._width);

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

void SharedImageHandleVk::ImportHandles(VkDevice device, VkPhysicalDevice physical_device, ExternalHandle::SharedImageInfo &&external_handles)
{
	this->_device = device;

	// Import Semaphores
	this->_semaphore_read = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_read);
	this->_semaphore_write = SharedImageHandleVk::ImportSemaphoreHandle(device, external_handles.handles.ext_write);

	// Create and allocate image memory
	this->_width = external_handles.width;
	this->_height = external_handles.height;
	this->_format = ExternalHandleVk::GetVkFormat(external_handles.format);

	VkExternalMemoryImageCreateInfo external_memory_image_create_info{VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO};
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
	memAllocInfo.memoryTypeIndex = VkHelpers::GetMemoryType(physical_device, memReqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);

	VK_CHECK(vkAllocateMemory(device, &memAllocInfo, nullptr, &this->_image_memory));

	VK_CHECK(vkBindImageMemory(device, this->_image, this->_image_memory, 0));

	// File descriptor ownership transferred to vulkan. Prevent clos on destructor call
	external_handles.handles.memory    = ExternalHandle::INVALID_VALUE;
	external_handles.handles.ext_read  = ExternalHandle::INVALID_VALUE;
	external_handles.handles.ext_write = ExternalHandle::INVALID_VALUE;
}

void SharedImageHandleVk::SetImageLayout(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImageLayout image_layout)
{
	VkHelpers::ImmediateSubmit(this->_device, graphics_queue, command_buffer,
	            [&](VkCommandBuffer image_command_buffer) {
		            VkHelpers::CmdPipelineMemoryBarrierColorImage(image_command_buffer, this->_image,
					                                              this->_image_layout, image_layout,
					                                              VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
	            },
	this->_semaphore_write);

	this->_image_layout = image_layout;
}

void SharedImageHandleVk::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image, VkImageLayout send_image_layout, VkFence fence)
{
	const VkOffset3D srcOffset[2] = {
	    {0, 0, 0},
	    {static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
	};

	return this->SendImageBlit(graphics_queue, command_buffer, send_image, send_image_layout, fence, srcOffset);
}

void SharedImageHandleVk::SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage send_image, VkImageLayout send_image_layout, VkFence fence, const VkOffset3D send_image_extent[2])
{
	const auto f = [&]() {
		constexpr VkImageLayout send_image_target_layout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL;
		if(send_image_layout != send_image_target_layout)
		{
			VkHelpers::CmdPipelineMemoryBarrierColorImage(command_buffer, this->_image,
			                                          this->_image_layout, send_image_target_layout,
			                                          VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
		}

		this->SendImageBlitCmd(command_buffer, send_image, send_image_target_layout, send_image_extent);

		if(send_image_layout != send_image_target_layout)
		{
			VkHelpers::CmdPipelineMemoryBarrierColorImage(command_buffer, this->_image,
			                                          this->_image_layout, send_image_layout,
			                                          VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
		}
	};

	this->TransceiveImageRecordCmdBuf(command_buffer,
	                                  //send_image, send_image_layout,
	                                  VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
	                                  f);

	// Wait until all read and write operations have completed before writing new images
	VkSemaphore wait_semaphores[] = {this->_semaphore_read, this->_semaphore_write};
	this->SubmitCommandBuffer(graphics_queue, command_buffer,
	                          wait_semaphores, 2,
	                          &this->_semaphore_write, 1,
	                          fence);
}

void SharedImageHandleVk::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image, VkImageLayout recv_image_layout, VkFence fence)
{
	const VkOffset3D dstOffset[2] = {
	    {0, 0, 0},
	    {static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
	};

	return this->RecvImageBlit(graphics_queue, command_buffer, recv_image, recv_image_layout, fence, dstOffset);
}

void SharedImageHandleVk::RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage recv_image, VkImageLayout recv_image_layout, VkFence fence, const VkOffset3D recv_image_extent[2])
{
	const auto f = [&]() {
		constexpr VkImageLayout recv_image_target_layout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
		if(recv_image_layout != recv_image_target_layout)
		{
			VkHelpers::CmdPipelineMemoryBarrierColorImage(command_buffer, this->_image,
			                                          this->_image_layout, recv_image_target_layout,
			                                          VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
		}

		this->ReceiveImageBlitCmd(command_buffer, recv_image, recv_image_target_layout, recv_image_extent);

		if(recv_image_layout != recv_image_target_layout)
		{
			VkHelpers::CmdPipelineMemoryBarrierColorImage(command_buffer, this->_image,
			                                          this->_image_layout, recv_image_layout,
			                                          VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);
		}
	};

	this->TransceiveImageRecordCmdBuf(command_buffer,
	                                  //recv_image, recv_image_layout,
	                                  VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
	                                  f);

	// Wait until write operation has completed before reading
	this->SubmitCommandBuffer(graphics_queue, command_buffer,
	                          &this->_semaphore_write, 1,
	                          &this->_semaphore_read, 1,
	                          fence);
}

void SharedImageHandleVk::ClearImage(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkClearColorValue clear_color, VkFence fence)
{
	const auto f = [&]() {
		this->ClearImageCmd(command_buffer, clear_color);
	};

	this->TransceiveImageRecordCmdBuf(command_buffer,
	                                  //recv_image, recv_image_layout,
	                                  VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
	                                  f);

	// Wait until all read and write operations have completed before writing new images
	VkSemaphore wait_semaphores[] = {this->_semaphore_read, this->_semaphore_write};
	this->SubmitCommandBuffer(graphics_queue, command_buffer,
	                          wait_semaphores, sizeof(wait_semaphores)/sizeof(wait_semaphores[0]),
	                          &this->_semaphore_write, 1,
	                          fence);
}

void SharedImageHandleVk::SendImageBlitCmd(VkCommandBuffer command_buffer, VkImage send_image, VkImageLayout send_image_layout)
{
	const VkOffset3D srcOffset[2] = {
	    {0, 0, 0},
	    {static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
	};

	return this->SendImageBlitCmd(command_buffer, send_image, send_image_layout, srcOffset);
}

void SharedImageHandleVk::SendImageBlitCmd(VkCommandBuffer command_buffer, VkImage send_image, VkImageLayout send_image_layout, const VkOffset3D send_image_extent[2])
{
	VkImageBlit region{};
	region.srcSubresource = CreateColorSubresourceLayer();
	region.dstSubresource = CreateColorSubresourceLayer();

	memcpy(region.srcOffsets, send_image_extent, sizeof(VkOffset3D[2]));

	region.dstOffsets[0].x = 0;
	region.dstOffsets[0].y = 0;
	region.dstOffsets[0].z = 0;

	region.dstOffsets[1].x = this->_width;
	region.dstOffsets[1].y = this->_height;
	region.dstOffsets[1].z = 1;

	vkCmdBlitImage(command_buffer, send_image, send_image_layout, this->_image, this->_image_layout, 1, &region, VK_FILTER_NEAREST);
}

void SharedImageHandleVk::ReceiveImageBlitCmd(VkCommandBuffer command_buffer, VkImage recv_image, VkImageLayout recv_image_layout)
{
	const VkOffset3D dstOffset[2] = {
	    {0, 0, 0},
	    {static_cast<int32_t>(this->_width), static_cast<int32_t>(this->_height), 1}
	};

	return this->ReceiveImageBlitCmd(command_buffer, recv_image, recv_image_layout, dstOffset);
}

void SharedImageHandleVk::ReceiveImageBlitCmd(VkCommandBuffer command_buffer, VkImage recv_image, VkImageLayout recv_image_layout, const VkOffset3D recv_image_extent[2])
{
	VkImageBlit region{};
	region.srcSubresource = CreateColorSubresourceLayer();
	region.dstSubresource = CreateColorSubresourceLayer();

	region.srcOffsets[0].x = 0;
	region.srcOffsets[0].y = 0;
	region.srcOffsets[0].z = 0;

	region.srcOffsets[1].x = this->_width;
	region.srcOffsets[1].y = this->_height;
	region.srcOffsets[1].z = 1;

	memcpy(region.dstOffsets, recv_image_extent, sizeof(VkOffset3D[2]));

	vkCmdBlitImage(command_buffer, this->_image, this->_image_layout, recv_image, recv_image_layout, 1, &region, VK_FILTER_NEAREST);
}

void SharedImageHandleVk::ClearImageCmd(VkCommandBuffer command_buffer, VkClearColorValue clear_color)
{
	VkHelpers::CmdClearColorImage(command_buffer, this->_image, clear_color, this->_image_layout);
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
	layer.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
	layer.baseArrayLayer = 0;
	layer.layerCount = 1;
	layer.mipLevel = 0;

	return layer;
}

VkSemaphore SharedImageHandleVk::ImportSemaphoreHandle(VkDevice device, ExternalHandle::TYPE semaphore_handle)
{
//	ExternalHandleVk::IMPORT_SEMAPHORE_INFO_KHR_T import_semapore_info = ExternalHandleVk::CreateImportSemaphoreInfoKHR(semaphore_handle);

//	VkSemaphoreCreateInfo semaphore_create_info{VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO, &import_semapore_info};
//	semaphore_create_info.flags = 0;

//	VkSemaphore semaphore;
//	VK_CHECK(vkCreateSemaphore(device, &semaphore_create_info, nullptr, &semaphore));

//	return semaphore;

	return ExternalHandleVk::CreateImportSemaphoreKHR(device, semaphore_handle);
}

void SharedImageHandleVk::TransceiveImageRecordCmdBuf(VkCommandBuffer command_buffer,
                                                      //VkImage transceive_image, VkImageLayout transceive_image_layout,
                                                      VkImageLayout shared_image_requested_layout,
                                                      transceive_fcn_t f)
{
	//naming it cmd for shorter writing
	VkCommandBuffer cmd = command_buffer;

	//begin the command buffer recording. We will use this command buffer exactly once, so we want to let Vulkan know that
	VkCommandBufferBeginInfo cmd_begin_info = {};
	cmd_begin_info.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
	cmd_begin_info.pNext = nullptr;

	cmd_begin_info.pInheritanceInfo = nullptr;
	cmd_begin_info.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	const VkImageLayout old_image_layout = this->_image_layout;
	if(this->_image_layout != shared_image_requested_layout)
	{
		VkHelpers::CmdPipelineMemoryBarrierColorImage(cmd, this->_image,
		                                          this->_image_layout, shared_image_requested_layout,
		                                          VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);

		this->_image_layout = shared_image_requested_layout;
	}

	//std::invoke(f, this, command_buffer, transceive_image, transceive_image_layout);
	std::invoke(f);

	if(old_image_layout != shared_image_requested_layout)
	{
		VkHelpers::CmdPipelineMemoryBarrierColorImage(cmd, this->_image,
		                                              shared_image_requested_layout, old_image_layout,
		                                              VK_ACCESS_NONE, VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT);

		this->_image_layout = old_image_layout;
	}

	VK_CHECK(vkEndCommandBuffer(cmd));
}

void SharedImageHandleVk::SubmitCommandBuffer(VkQueue graphics_queue, VkCommandBuffer command_buffer,
                                              VkSemaphore *wait_semaphores, uint32_t num_wait_semaphores,
                                              VkSemaphore *signal_semaphores, uint32_t num_signal_semaphores,
                                              VkFence fence)
{
	//prepare the submission to the queue.
	//we want to wait on the _presentSemaphore, as that semaphore is signaled when the swapchain is ready
	//we will signal the _renderSemaphore, to signal that rendering has finished

	VkSubmitInfo submit = {};
	submit.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit.pNext = nullptr;

	VkPipelineStageFlags wait_stage = VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT;
	submit.pWaitDstStageMask = &wait_stage;

//	submit.waitSemaphoreCount = num_wait_semaphores;
//	submit.pWaitSemaphores = wait_semaphores;

	submit.signalSemaphoreCount = num_signal_semaphores;
	submit.pSignalSemaphores = signal_semaphores;

	submit.commandBufferCount = 1;
	submit.pCommandBuffers = &command_buffer;

	//submit command buffer to the queue and execute it.
	// if set, fence may block until the graphic commands finish execution
	VK_CHECK(vkQueueSubmit(graphics_queue, 1, &submit, fence));

	// Wait for the fence to signal that command buffer has finished executing
	if(fence != VK_NULL_HANDLE)
	{
		VK_CHECK(vkWaitForFences(this->_device, 1, &fence, VK_TRUE, VkHelpers::DEFAULT_FENCE_TIMEOUT));
		VK_CHECK(vkResetFences(this->_device, 1, &fence));
	}
}
