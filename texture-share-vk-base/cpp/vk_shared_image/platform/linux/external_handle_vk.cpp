#include "external_handle_vk.h"

// #include "texture_share_vk/logging.h"
#include "vk_shared_image/vk_helpers.h"

// PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR
// 	ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = nullptr;
// PFN_vkGetMemoryFdKHR ExternalHandleVk::pvkGetMemoryFdKHR                 = nullptr;
// PFN_vkGetSemaphoreFdKHR ExternalHandleVk::pvkGetSemaphoreFdKHR           = nullptr;
// PFN_vkImportSemaphoreFdKHR ExternalHandleVk::pvkImportSemaphoreFdKHR     = nullptr;

// VkExportSemaphoreCreateInfo ExternalHandleVk::export_semaphore_create_info{};
// VkSemaphoreTypeCreateInfo ExternalHandleVk::semaphore_type_create_info{};
// VkSemaphoreCreateInfo ExternalHandleVk::semaphore_create_info{};

ExternalHandleVk::ExternalHandleVk(VkInstance instance, VkPhysicalDevice physical_device)
{
	this->LoadVulkanHandleExtensions(instance);
	this->LoadCompatibleSemaphorePropsInfo(physical_device);
}

bool ExternalHandleVk::LoadVulkanHandleExtensions(VkInstance instance)
{
	// if(!ExternalHandleVk::pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR)
	this->pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR =
		(PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR)vkGetInstanceProcAddr(
			instance, "vkGetPhysicalDeviceExternalSemaphorePropertiesKHR");

	// if(!ExternalHandleVk::pvkGetMemoryFdKHR)
	this->pvkGetMemoryFdKHR = (PFN_vkGetMemoryFdKHR)vkGetInstanceProcAddr(instance, "vkGetMemoryFdKHR");

	// if(!ExternalHandleVk::pvkGetSemaphoreFdKHR)
	this->pvkGetSemaphoreFdKHR = (PFN_vkGetSemaphoreFdKHR)vkGetInstanceProcAddr(instance, "vkGetSemaphoreFdKHR");

	// if(!ExternalHandleVk::pvkImportSemaphoreFdKHR)
	this->pvkImportSemaphoreFdKHR =
		(PFN_vkImportSemaphoreFdKHR)vkGetInstanceProcAddr(instance, "vkImportSemaphoreFdKHR");

	return this->pvkGetMemoryFdKHR && this->pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR &&
	       this->pvkGetSemaphoreFdKHR && this->pvkImportSemaphoreFdKHR;
}

bool ExternalHandleVk::LoadCompatibleSemaphorePropsInfo(VkPhysicalDevice physical_device)
{
	VkExternalSemaphoreHandleTypeFlagBits flags[] = {
		VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT, VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_BIT,
		VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_KMT_BIT, VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_D3D12_FENCE_BIT,
		VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_SYNC_FD_BIT};

	VkPhysicalDeviceExternalSemaphoreInfo zzzz{VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_EXTERNAL_SEMAPHORE_INFO, nullptr,
	                                           VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM};
	VkExternalSemaphoreProperties aaaa{
		VK_STRUCTURE_TYPE_EXTERNAL_SEMAPHORE_PROPERTIES, nullptr, VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM,
		VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_FLAG_BITS_MAX_ENUM, VK_EXTERNAL_SEMAPHORE_FEATURE_FLAG_BITS_MAX_ENUM};

	bool found = false;
	VkExternalSemaphoreHandleTypeFlagBits compatable_semaphore_type;
	for(size_t i = 0; i < sizeof(flags) / sizeof(flags[0]); i++)
	{
		zzzz.handleType = flags[i];
		this->pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR(physical_device, &zzzz, &aaaa);
		if(aaaa.compatibleHandleTypes & flags[i] &&
		   aaaa.externalSemaphoreFeatures & VK_EXTERNAL_SEMAPHORE_FEATURE_EXPORTABLE_BIT)
		{
			compatable_semaphore_type = flags[i];
			found                     = true;
			break;
		}
	}

	if(!found)
		return false;

	this->export_semaphore_create_info = {VK_STRUCTURE_TYPE_EXPORT_SEMAPHORE_CREATE_INFO, nullptr,
	                                      VkExternalSemaphoreHandleTypeFlags(compatable_semaphore_type)};
	//	ExternalHandleVk::semaphore_type_create_info = {
	//	    VK_STRUCTURE_TYPE_SEMAPHORE_TYPE_CREATE_INFO,
	//	    &export_semaphore_create_info,
	//	    VK_SEMAPHORE_TYPE_TIMELINE,
	//	    0};
	this->semaphore_create_info = {VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO, &export_semaphore_create_info, 0};

	return true;
}

ExternalHandleVk::SEMAPHORE_GET_INFO_T ExternalHandleVk::CreateSemaphoreGetInfoKHR(
	VkExternalSemaphoreHandleTypeFlagBits compatible_type)
{
	return VkSemaphoreGetFdInfoKHR{VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR, nullptr, VK_NULL_HANDLE,
	                               compatible_type};
}

ExternalHandleVk::MEMORY_GET_INFO_T ExternalHandleVk::CreateMemoryGetInfoKHR(VkDeviceMemory memory)
{
	return VkMemoryGetFdInfoKHR{VK_STRUCTURE_TYPE_MEMORY_GET_FD_INFO_KHR, nullptr, memory,
	                            VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT};
}

void ExternalHandleVk::GetMemoryKHR(VkDevice device, const MEMORY_GET_INFO_T *info, ExternalHandle::TYPE *memory) const
{
	VK_CHECK(ExternalHandleVk::pvkGetMemoryFdKHR(device, info, memory));
}

VkSemaphore ExternalHandleVk::CreateExternalSemaphore(VkDevice device) const
{
	VkSemaphore semaphore{VK_NULL_HANDLE};

	// Create semaphores. Ensure ExternalHandleVk::FindCompatibleSemaphoreProps() was already run before
	VK_CHECK(vkCreateSemaphore(device, &this->ExternalSemaphoreCreateInfo(), nullptr, &semaphore));

	return semaphore;
}

ExternalHandle::TYPE ExternalHandleVk::GetSemaphoreKHR(VkDevice device, VkSemaphore semaphore) const
{
	VkSemaphoreGetFdInfoKHR semaphoreGetFdInfo{
		VK_STRUCTURE_TYPE_SEMAPHORE_GET_FD_INFO_KHR, nullptr, semaphore,
		(VkExternalSemaphoreHandleTypeFlagBits)this->export_semaphore_create_info.handleTypes};

	ExternalHandle::TYPE fd;
	VK_CHECK(this->pvkGetSemaphoreFdKHR(device, &semaphoreGetFdInfo, &fd));
	return fd;
}

ExternalHandleVk::IMPORT_MEMORY_INFO_KHR_T ExternalHandleVk::CreateImportMemoryInfoKHR(ExternalHandle::TYPE handle)
{
	return VkImportMemoryFdInfoKHR{VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR, nullptr,
	                               VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT, handle};
}

// ExternalHandleVk::IMPORT_SEMAPHORE_INFO_KHR_T ExternalHandleVk::CreateImportSemaphoreInfoKHR(ExternalHandle::TYPE
// handle)
//{
//	VkImportSemaphoreFdInfoKHR import_semaphore_info{VK_STRUCTURE_TYPE_IMPORT_SEMAPHORE_FD_INFO_KHR, nullptr};
//	import_semaphore_info.fd = handle;

//	return import_semaphore_info;
//}

VkSemaphore ExternalHandleVk::CreateImportSemaphoreKHR(VkDevice device, ExternalHandle::TYPE handle) const
{
	VkSemaphore semaphore = this->CreateExternalSemaphore(device);

	VkImportSemaphoreFdInfoKHR import_semaphore_info{VK_STRUCTURE_TYPE_IMPORT_SEMAPHORE_FD_INFO_KHR, nullptr};
	import_semaphore_info.fd = handle;
	import_semaphore_info.handleType =
		(VkExternalSemaphoreHandleTypeFlagBits)this->export_semaphore_create_info.handleTypes;
	import_semaphore_info.semaphore = semaphore;

	VK_CHECK(this->pvkImportSemaphoreFdKHR(device, &import_semaphore_info));

	return semaphore;
}
