#pragma once

#include "platform/linux/external_handle_vk.h"
#include "vk_shared_image/vk_setup.h"
#include <memory>
#include <vulkan/vulkan_core.h>

class VkSetupWrapper : public VkSetup
{
	public:
	VkSetupWrapper()  = default;
	~VkSetupWrapper() = default;

	void initialize_vulkan()
	{
		return this->InitializeVulkan();
	}

	void import_vulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device, VkQueue graphics_queue,
	                   uint32_t graphics_queue_index)
	{
		return this->InitializeVulkan(instance, device, physical_device, graphics_queue, graphics_queue_index, true);
	}

	void import_vulkan_as_owned(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
	                            VkQueue graphics_queue, uint32_t graphics_queue_index)
	{
		return this->InitializeVulkan(instance, device, physical_device, graphics_queue, graphics_queue_index, false);
	}

	bool is_vulkan_initialized() const
	{
		return this->IsVulkanInitialized();
	}

	void cleanup_vulkan()
	{
		return this->CleanupVulkan();
	}

	constexpr const ExternalHandleVk &get_external_handle_info() const
	{
		return this->ExternalHandle();
	}

	constexpr VkInstance get_vk_instance() const
	{
		return this->VulkanInstance();
	}

	constexpr VkDevice get_vk_device() const
	{
		return this->VulkanDevice();
	}

	constexpr VkPhysicalDevice get_vk_physical_device() const
	{
		return this->VulkanPhysicalDevice();
	}

	constexpr VkQueue get_vk_queue() const
	{
		return this->GraphicsQueue();
	}

	constexpr uint32_t get_vk_queue_index() const
	{
		return this->GraphicsQueueIndex();
	}

	constexpr VkCommandPool get_vk_command_pool() const
	{
		return this->CommandPool();
	}

	constexpr VkCommandBuffer get_vk_command_buffer() const
	{
		return this->CommandBuffer();
	}

	VkFence create_vk_fence()
	{
		VkFence fence = VK_NULL_HANDLE;

		VkFenceCreateInfo create_info{VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, nullptr, 0};
		vkCreateFence(this->VulkanDevice(), &create_info, nullptr, &fence);

		return fence;
	}

	void destroy_vk_fence(VkFence fence)
	{
		vkDestroyFence(this->VulkanDevice(), fence, nullptr);
	}
};

std::unique_ptr<VkSetupWrapper> vk_setup_new();
