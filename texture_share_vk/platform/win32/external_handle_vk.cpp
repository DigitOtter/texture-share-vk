#include "external_handle_vk.h"

#include "texture_share_vk/logging.h"
#include "texture_share_vk/vk_helpers.h"


PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = nullptr;
PFN_vkGetMemoryWin32HandleKHR ExternalHandleVk::pvkGetMemoryWin32HandleKHR = nullptr;
PFN_vkGetSemaphoreWin32HandleKHR ExternalHandleVk::pvkGetSemaphoreWin32HandleKHR = nullptr;
PFN_vkImportSemaphoreWin32HandleKHR ExternalHandleVk::pvkImportSemaphoreWin32HandleKHR = nullptr;

VkExportSemaphoreCreateInfo ExternalHandleVk::export_semaphore_create_info{};
VkSemaphoreTypeCreateInfo   ExternalHandleVk::semaphore_type_create_info{};
VkSemaphoreCreateInfo       ExternalHandleVk::semaphore_create_info{};


VkFormat ExternalHandleVk::GetVkFormat(ExternalHandle::ImageFormat format)
{
	switch(format)
	{
		case ExternalHandle::ImageFormat::R8G8B8A8:
			return VK_FORMAT_R8G8B8A8_UNORM;
		default:
			return VK_FORMAT_MAX_ENUM;
	}
}

ExternalHandle::ImageFormat ExternalHandleVk::GetImageFormat(VkFormat vk_format)
{
	switch(vk_format)
	{
		case VK_FORMAT_R8G8B8A8_UNORM:
			return ExternalHandle::ImageFormat::R8G8B8A8;
		default:
			return ExternalHandle::ImageFormat::IMAGE_FORMAT_MAX_ENUM;
	}
}

bool ExternalHandleVk::LoadVulkanHandleExtensions(VkInstance instance)
{
	if(!ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR)
		ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = (PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR)vkGetInstanceProcAddr(instance, "vkGetPhysicalDeviceExternalSemaphorePropertiesKHR");

	if(!ExternalHandleVk::pvkGetMemoryWin32HandleKHR)
		ExternalHandleVk::pvkGetMemoryWin32HandleKHR = (PFN_vkGetMemoryWin32HandleKHR)vkGetInstanceProcAddr(instance, "vkGetMemoryWin32HandleKHR");

	if(!ExternalHandleVk::pvkGetSemaphoreWin32HandleKHR)
		ExternalHandleVk::pvkGetSemaphoreWin32HandleKHR = (PFN_vkGetSemaphoreWin32HandleKHR)vkGetInstanceProcAddr(instance, "vkGetSemaphoreWin32HandleKHR");

	if(!ExternalHandleVk::pvkImportSemaphoreWin32HandleKHR)
		ExternalHandleVk::pvkImportSemaphoreWin32HandleKHR = (PFN_vkImportSemaphoreWin32HandleKHR)vkGetInstanceProcAddr(instance, "vkImportSemaphoreWin32HandleKHR");

	return ExternalHandleVk::pvkGetMemoryWin32HandleKHR &&
	        ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR &&
	        ExternalHandleVk::pvkGetSemaphoreWin32HandleKHR &&
	        ExternalHandleVk::pvkImportSemaphoreWin32HandleKHR;
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
	for (size_t i = 0; i < sizeof(flags)/sizeof(flags[0]); i++)
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
//	ExternalHandleVk::semaphore_type_create_info = {
//	    VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO,
//	    &export_semaphore_create_info,
//	    VK_SEMAPHORE_TYPE_TIMELINE,
//	    0};
	ExternalHandleVk::semaphore_create_info = {VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
	                                          &export_semaphore_create_info, 0};

	return true;
}

ExternalHandleVk::SEMAPHORE_GET_INFO_T ExternalHandleVk::CreateSemaphoreGetInfoKHR(VkExternalSemaphoreHandleTypeFlagBits compatible_type)
{
	return VkSemaphoreGetWin32HandleInfoKHR{
		VK_STRUCTURE_TYPE_SEMAPHORE_GET_WIN32_HANDLE_INFO_KHR, nullptr,
		        VK_NULL_HANDLE, compatible_type};
}

ExternalHandleVk::MEMORY_GET_INFO_T ExternalHandleVk::CreateMemoryGetInfoKHR(VkDeviceMemory memory)
{
	return VkMemoryGetWin32HandleInfoKHR{VK_STRUCTURE_TYPE_MEMORY_GET_WIN32_HANDLE_INFO_KHR, nullptr,
		        memory,
		        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT_KHR};
}

void ExternalHandleVk::GetMemoryKHR(VkDevice device, const MEMORY_GET_INFO_T *info, ExternalHandle::TYPE *memory)
{
	VK_CHECK(ExternalHandleVk::pvkGetMemoryWin32HandleKHR(device, info, memory));
}

VkSemaphore ExternalHandleVk::CreateExternalSemaphore(VkDevice device)
{
	VkSemaphore semaphore{VK_NULL_HANDLE};

	// Create semaphores. Ensure ExternalHandleVk::FindCompatibleSemaphoreProps() was already run before
	VK_CHECK(vkCreateSemaphore(device, &ExternalHandleVk::ExternalSemaphoreCreateInfo(), nullptr,
	                           &semaphore));

	return semaphore;
}

ExternalHandle::TYPE ExternalHandleVk::GetSemaphoreKHR(VkDevice device, VkSemaphore semaphore)
{
	VkSemaphoreGetWin32HandleInfoKHR semaphoreGetFdInfo{
		VK_STRUCTURE_TYPE_SEMAPHORE_GET_WIN32_HANDLE_INFO_KHR, nullptr,
		semaphore, (VkExternalSemaphoreHandleTypeFlagBits)export_semaphore_create_info.handleTypes};

	ExternalHandle::TYPE fd;
	VK_CHECK(ExternalHandleVk::pvkGetSemaphoreWin32HandleKHR(device, &semaphoreGetFdInfo, &fd));
	return fd;
}

ExternalHandleVk::IMPORT_MEMORY_INFO_KHR_T ExternalHandleVk::CreateImportMemoryInfoKHR(ExternalHandle::TYPE handle)
{
	return VkImportMemoryWin32HandleInfoKHR{VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR, nullptr,
		        VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT, handle};
}

VkSemaphore ExternalHandleVk::CreateImportSemaphoreKHR(VkDevice device, ExternalHandle::TYPE handle)
{
	VkSemaphore semaphore = ExternalHandleVk::CreateExternalSemaphore(device);

	VkImportSemaphoreWin32HandleInfoKHR import_semaphore_info{VK_STRUCTURE_TYPE_IMPORT_SEMAPHORE_WIN32_HANDLE_INFO_KHR, nullptr};
	import_semaphore_info.handle = handle;
	import_semaphore_info.handleType = (VkExternalSemaphoreHandleTypeFlagBits)ExternalHandleVk::export_semaphore_create_info.handleTypes;
	import_semaphore_info.semaphore = semaphore;

	VK_CHECK(ExternalHandleVk::pvkImportSemaphoreWin32HandleKHR(device, &import_semaphore_info));

	return semaphore;
}
