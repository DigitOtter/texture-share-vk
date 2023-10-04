#include "vk_setup.h"

#include "platform/external_handle_vk.h"

VkSetup::~VkSetup()
{
	this->CleanupVulkan();
}

void VkSetup::InitializeVulkan()
{
	this->_vk_struct = VkHelpers::CreateTextureShareVkInstance();

	// Destroy vulkan if not importing
	this->_cleanup_vk = true;

	ExternalHandleVk::LoadVulkanHandleExtensions(this->_vk_struct.instance);
	ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(this->_vk_struct.physical_device);

	this->InitCommandBuffer();
}

void VkSetup::InitializeVulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
                               VkQueue graphics_queue, uint32_t graphics_queue_index, bool import_only)
{
	this->_vk_struct                      = {};
	this->_vk_struct.instance             = instance;
	this->_vk_struct.device               = device;
	this->_vk_struct.physical_device      = physical_device;
	this->_vk_struct.graphics_queue       = graphics_queue;
	this->_vk_struct.graphics_queue_index = graphics_queue_index;

	// Destroy vulkan if not importing
	this->_cleanup_vk = !import_only;

	ExternalHandleVk::LoadVulkanHandleExtensions(this->_vk_struct.instance);
	ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(this->_vk_struct.physical_device);

	this->InitCommandBuffer();
}

void VkSetup::CleanupVulkan()
{
	if(this->_vk_struct.instance != VK_NULL_HANDLE)
	{
		VkHelpers::CleanupCommandBuffer(this->_vk_struct.device, this->_command_pool, this->_command_buffer);
		VkHelpers::CleanupCommandPool(this->_vk_struct.device, this->_command_pool);
		VkHelpers::CleanupTextureShareVkInstance(this->_vk_struct, this->_cleanup_vk, this->_cleanup_vk);

		this->_vk_struct.device   = VK_NULL_HANDLE;
		this->_vk_struct.instance = VK_NULL_HANDLE;
	}
}

bool VkSetup::IsVulkanInitialized() const
{
	return this->_vk_struct.instance != VK_NULL_HANDLE;
}

void VkSetup::InitCommandBuffer()
{
	this->_command_pool = VkHelpers::CreateCommandPool(this->_vk_struct.device, this->_vk_struct.graphics_queue_index);
	this->_command_buffer = VkHelpers::CreateCommandBuffer(this->_vk_struct.device, this->_command_pool);
}
