#pragma once

#include "vk_shared_image/platform/linux/external_handle.h"
#include "vk_shared_image/vk_shared_image.h"
#include <cstdint>
#include <memory>
#include <vulkan/vulkan_core.h>

class ShareHandlesWrapper : public ExternalHandle::ShareHandles
{
	public:
	ShareHandlesWrapper()  = default;
	~ShareHandlesWrapper() = default;

	ShareHandlesWrapper(ExternalHandle::ShareHandles &&handles)
		: ExternalHandle::ShareHandles(std::move(handles))
	{}

	constexpr ExternalHandle::TYPE get_memory_handle() const
	{
		return this->memory;
	}

	ExternalHandle::TYPE release_memory_handle()
	{
		const auto res = std::move(this->memory);
		this->memory   = ExternalHandle::INVALID_VALUE;

		return res;
	}
};

class VkSharedImageWrapper : public VkSharedImage
{
	public:
	VkSharedImageWrapper()  = default;
	~VkSharedImageWrapper() = default;

	static void initialize_vulkan(VkInstance instance, VkPhysicalDevice physical_device)
	{
		return VkSharedImage::InitializeVulkan(instance, physical_device);
	}

	void initialize(VkDevice device, VkPhysicalDevice physical_device, VkQueue queue, VkCommandBuffer command_buffer,
	                uint32_t width, uint32_t height, VkFormat format, uint32_t id)
	{
		return this->Initialize(device, physical_device, queue, command_buffer, width, height, format, id);
	}

	void cleanup()
	{
		return this->Cleanup();
	}

	void import_from_handle(VkDevice device, VkPhysicalDevice physical_device,
	                        std::unique_ptr<ShareHandlesWrapper> share_handles, const SharedImageData &image_data)
	{
		return this->ImportFromHandle(device, physical_device, std::move(*share_handles), image_data);
	}

	void send_image_blit_with_extents(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
	                                  VkImageLayout dst_image_layout, VkFence fence,
	                                  const VkOffset3D dst_image_extent[2])
	{
		return this->SendImageBlit(graphics_queue, command_buffer, dst_image, dst_image_layout, fence,
		                           dst_image_extent);
	}

	void send_image_blit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
	                     VkImageLayout dst_image_layout, VkFence fence)
	{
		return this->SendImageBlit(graphics_queue, command_buffer, dst_image, dst_image_layout, fence);
	}

	void recv_image_blit_with_extents(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
	                                  VkImageLayout src_image_layout, VkFence fence,
	                                  const VkOffset3D src_image_extent[2])
	{
		return this->RecvImageBlit(graphics_queue, command_buffer, src_image, src_image_layout, fence,
		                           src_image_extent);
	}

	void recv_image_blit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
	                     VkImageLayout src_image_layout, VkFence fence)
	{
		return this->RecvImageBlit(graphics_queue, command_buffer, src_image, src_image_layout, fence);
	}

	std::unique_ptr<ShareHandlesWrapper> export_handles()
	{
		return std::make_unique<ShareHandlesWrapper>(this->ExportHandles());
	}

	constexpr const SharedImageData &get_image_data() const
	{
		return this->ImageData();
	}

	constexpr SharedImageData &get_image_data_mut()
	{
		return this->ImageData();
	}
};

std::unique_ptr<VkSharedImageWrapper> vk_shared_image_new();
std::unique_ptr<ShareHandlesWrapper> vk_share_handles_new();

void initialize_vulkan(VkInstance instance, VkPhysicalDevice physical_device);
