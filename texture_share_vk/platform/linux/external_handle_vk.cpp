#include "external_handle_vk.h"

#include "texture_share_vk/logging.h"


PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = nullptr;
PFN_vkGetMemoryFdKHR ExternalHandleVk::pvkGetMemoryFdKHR = nullptr;

VkExportSemaphoreCreateInfo ExternalHandleVk::export_semaphore_create_info{};
VkSemaphoreCreateInfo       ExternalHandleVk::semaphore_create_info{};


bool ExternalHandleVk::LoadVulkanHandleExtensions(VkInstance instance)
{
	if(!ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR)
		ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = (PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR)vkGetInstanceProcAddr(instance, "vkGetPhysicalDeviceExternalSemaphorePropertiesKHR");

	if(!ExternalHandleVk::pvkGetMemoryFdKHR)
		ExternalHandleVk::pvkGetMemoryFdKHR = (PFN_vkGetMemoryFdKHR)vkGetInstanceProcAddr(instance, "vkGetMemoryFdKHR");

	return ExternalHandleVk::pvkGetMemoryFdKHR && ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR;
}

bool ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(VkPhysicalDevice physical_device)
{
	VkExternalSemaphoreHandleTypeFlagBits flags[] = {
	    VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT,
	    VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_BIT,
	    VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_KMT_BIT,
	    VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_D3D12_FENCE_BIT,
	    VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_SYNC_FD_BIT};

	VkPhysicalDeviceExternalSemaphoreInfo zzzz{VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_SEMAPHORE_INFO,
		        nullptr, VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM};
	VkExternalSemaphoreProperties aaaa{VK_STRUCTURE_TYPE_EXTERNAL_SEMAPHORE_PROPERTIES,
		        nullptr, VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM,
		        VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM, VK_EXTERNAL_SEMAPHORE_FEATURE_FLAG_BITS_MAX_ENUM};

	bool found = false;
	VkExternalSemaphoreHandleTypeFlagBits compatable_semaphore_type;
	for (size_t i = 0; i < 5; i++)
	{
		zzzz.handleType = flags[i];
		ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR(physical_device, &zzzz, &aaaa);
		if (aaaa.compatibleHandleTypes & flags[i] &&
		        aaaa.externalSemaphoreFeatures & VK_EXTERNAL_SEMAPHORE_FEATURE_EXPORTABLE_BIT)
		{
			compatable_semaphore_type = flags[i];
			found                     = true;
			break;
		}
	}

	if (!found)
		return false;

	ExternalHandleVk::export_semaphore_create_info = {
	    VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO, nullptr,
	    VkExternalSemaphoreHandleTypeFlags(compatable_semaphore_type)};
	ExternalHandleVk::semaphore_create_info = {VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
	                                          &export_semaphore_create_info, 0};

	return true;
}

ExternalHandleVk::SEMAPHORE_GET_INFO_T ExternalHandleVk::CreateSemaphoreGetInfoKHR(VkExternalSemaphoreHandleTypeFlagBits compatable_type)
{
	return VkSemaphoreGetFdInfoKHR{
		VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR, nullptr,
		        VK_NULL_HANDLE, compatable_type};
}

ExternalHandleVk::MEMORY_GET_INFO_T ExternalHandleVk::CreateMemoryGetInfoKHR(VkDeviceMemory memory)
{
	return VkMemoryGetFdInfoKHR{VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR, nullptr,
		        memory,
		        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT};
}

void ExternalHandleVk::GetMemoryKHR(VkDevice device, MEMORY_GET_INFO_T *info, ExternalHandle::TYPE *memory)
{
	VK_CHECK(ExternalHandleVk::pvkGetMemoryFdKHR(device, info, memory));
}
