#ifndef EXTERNAL_HANDLE_VK_H
#define EXTERNAL_HANDLE_VK_H

// #include "texture_share_vk/platform/platform.h"
#include "external_handle.h"

// #define VK_USE_PLATFORM_XLIB_KHR
// #include "volk.h"

#include <string_view>
#include <vulkan/vulkan.hpp>

class ExternalHandleVk
{
	PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR = nullptr;
	PFN_vkGetMemoryFdKHR pvkGetMemoryFdKHR                                                                   = nullptr;
	PFN_vkGetSemaphoreFdKHR pvkGetSemaphoreFdKHR                                                             = nullptr;
	PFN_vkImportSemaphoreFdKHR pvkImportSemaphoreFdKHR                                                       = nullptr;

	VkExportSemaphoreCreateInfo export_semaphore_create_info{};
	VkSemaphoreTypeCreateInfo semaphore_type_create_info{};
	VkSemaphoreCreateInfo semaphore_create_info{};

	public:
	static constexpr std::string_view HOST_MEMORY_EXTENSION_NAME    = VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME;
	static constexpr std::string_view HOST_SEMAPHORE_EXTENSION_NAME = VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME;

	ExternalHandleVk() = default;
	ExternalHandleVk(VkInstance instance, VkPhysicalDevice physical_device);

	bool LoadVulkanHandleExtensions(VkInstance instance);
	bool LoadCompatibleSemaphorePropsInfo(VkPhysicalDevice physical_device);

	using SEMAPHORE_GET_INFO_T = VkSemaphoreGetFdInfoKHR;
	static SEMAPHORE_GET_INFO_T CreateSemaphoreGetInfoKHR(VkExternalSemaphoreHandleTypeFlagBits compatible_type);

	using MEMORY_GET_INFO_T = VkMemoryGetFdInfoKHR;
	static MEMORY_GET_INFO_T CreateMemoryGetInfoKHR(VkDeviceMemory memory);
	void GetMemoryKHR(VkDevice device, const MEMORY_GET_INFO_T *info, ExternalHandle::TYPE *memory) const;

	static constexpr auto EXTERNAL_MEMORY_HANDLE_TYPE = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;

	constexpr const VkSemaphoreCreateInfo &ExternalSemaphoreCreateInfo() const
	{
		return ExternalHandleVk::semaphore_create_info;
	}

	VkSemaphore CreateExternalSemaphore(VkDevice device) const;

	ExternalHandle::TYPE GetSemaphoreKHR(VkDevice device, VkSemaphore semaphore) const;

	using IMPORT_MEMORY_INFO_KHR_T = VkImportMemoryFdInfoKHR;
	static IMPORT_MEMORY_INFO_KHR_T CreateImportMemoryInfoKHR(ExternalHandle::TYPE handle);

	//		using IMPORT_SEMAPHORE_INFO_KHR_T = VkImportSemaphoreFdInfoKHR;
	//		static IMPORT_SEMAPHORE_INFO_KHR_T CreateImportSemaphoreInfoKHR(ExternalHandle::TYPE handle);

	VkSemaphore CreateImportSemaphoreKHR(VkDevice device, ExternalHandle::TYPE handle) const;

	private:
};

#endif // EXTERNAL_HANDLE_VK_H
