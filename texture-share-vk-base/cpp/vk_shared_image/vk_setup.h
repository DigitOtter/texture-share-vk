#pragma once

#include "vk_helpers.h"
#include "platform/linux/external_handle_vk.h"

#include <unistd.h>

class VkSetup
{
	public:
	static constexpr VkFormat DEFAULT_FORMAT = VK_FORMAT_R8G8B8A8_UNORM;

	VkSetup() = default;
	~VkSetup();

	void InitializeVulkan();
	void InitializeVulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
	                      VkQueue graphics_queue, uint32_t graphics_queue_index, bool import_only = true);
	void CleanupVulkan();

	bool IsVulkanInitialized() const;

	constexpr VkInstance VulkanInstance() const
	{
		return this->_vk_struct.instance;
	}

	constexpr VkDevice VulkanDevice() const
	{
		return this->_vk_struct.device;
	}

	constexpr VkPhysicalDevice VulkanPhysicalDevice() const
	{
		return this->_vk_struct.physical_device;
	}

	constexpr VkQueue GraphicsQueue() const
	{
		return this->_vk_struct.graphics_queue;
	}

	constexpr uint32_t GraphicsQueueIndex() const
	{
		return this->_vk_struct.graphics_queue_index;
	}

	constexpr VkCommandPool CommandPool() const
	{
		return this->_command_pool;
	}

	constexpr VkCommandBuffer CommandBuffer() const
	{
		return this->_command_buffer;
	}

	constexpr const ExternalHandleVk &ExternalHandle() const
	{
		return this->_external_handle;
	}

	private:
	VkHelpers::TextureShareVkStruct _vk_struct{};
	bool _cleanup_vk = true;

	ExternalHandleVk _external_handle;

	VkCommandPool _command_pool{VK_NULL_HANDLE};
	VkCommandBuffer _command_buffer{VK_NULL_HANDLE};

	void InitCommandBuffer();
};
