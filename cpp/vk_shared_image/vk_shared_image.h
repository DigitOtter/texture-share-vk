#pragma once

#include "vk_helpers.h"
#include "vk_shared_image/platform/linux/external_handle.h"

#include <cstdint>
#include <memory>
#include <vulkan/vulkan.h>
#include <vulkan/vulkan_core.h>

struct SharedImageData
{
	uint32_t Id                 = 0;
	uint32_t Width              = 0;
	uint32_t Height             = 0;
	VkFormat Format             = VK_FORMAT_UNDEFINED;
	VkDeviceSize AllocationSize = 0;
};

class VkSharedImage
{
	public:
	VkSharedImage() = default;
	~VkSharedImage();

	static void InitializeVulkan();

	void Initialize(VkDevice device, VkPhysicalDevice physical_device, uint32_t width, uint32_t height, VkFormat format,
	                uint32_t id = 0);
	void Cleanup();

	void ImportFromHandle(VkDevice device, VkPhysicalDevice physical_device,
	                      ExternalHandle::ShareHandles &&share_handles, const SharedImageData &image_data);

	static VkImageSubresourceLayers CreateColorSubresourceLayer();
	void SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
	                   VkImageLayout dst_image_layout, VkFence fence, const VkOffset3D dst_image_extent[2]);

	void SendImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage dst_image,
	                   VkImageLayout dst_image_layout, VkFence fence);

	void RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
	                   VkImageLayout src_image_layout, VkFence fence, const VkOffset3D src_image_extent[2]);

	void RecvImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
	                   VkImageLayout src_image_layout, VkFence fence);


	ExternalHandle::ShareHandles ExportHandles();

	constexpr const SharedImageData &ImageData() const
	{
		return this->_data;
	}

	constexpr SharedImageData &ImageData()
	{
		return this->_data;
	}

	private:
	VkImage _image        = VK_NULL_HANDLE;
	VkImageLayout _layout = VK_IMAGE_LAYOUT_UNDEFINED;

	SharedImageData _data;

	VkDevice _device       = VK_NULL_HANDLE;
	VkDeviceMemory _memory = VK_NULL_HANDLE;

	void ImageBlit(VkQueue graphics_queue, VkCommandBuffer command_buffer, VkImage src_image,
	               VkImageLayout src_image_layout, const VkOffset3D src_image_extent[2], VkImage dst_image,
	               VkImageLayout dst_image_layout, const VkOffset3D dst_image_extent[2], VkFence fence);
};

std::unique_ptr<VkSharedImage> vk_shared_image_new();
