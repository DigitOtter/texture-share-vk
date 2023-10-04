#include "vk_shared_image_wrapper.h"
#include "vk_shared_image/platform/linux/external_handle.h"
#include <memory>

std::unique_ptr<VkSharedImageWrapper> vk_shared_image_new()
{
	return std::make_unique<VkSharedImageWrapper>();
}

std::unique_ptr<ShareHandlesWrapper> vk_share_handles_new()
{
	return std::make_unique<ShareHandlesWrapper>();
}

void initialize_vulkan(VkInstance instance, VkPhysicalDevice physical_device)
{
	return VkSharedImageWrapper::initialize_vulkan(instance, physical_device);
}
