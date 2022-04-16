#ifndef EXTERNAL_HANDLE_VK_H
#define EXTERNAL_HANDLE_VK_H

#include "texture_share_vk/platform/platform.h"

//#define VK_USE_PLATFORM_XLIB_KHR
//#include "volk.h"

#include <vulkan/vulkan.hpp>
#include <string_view>


class ExternalHandleVk
{
		static PFN_vkGetPhysicalDeviceExternalSemaphorePropertiesKHR pvkGetPhysicalDeviceExternalSemaphorePropertiesKHR;
		static PFN_vkGetMemoryFdKHR pvkGetMemoryFdKHR;
		static PFN_vkGetSemaphoreFdKHR pvkGetSemaphoreFdKHR;
		static PFN_vkImportSemaphoreFdKHR pvkImportSemaphoreFdKHR;

		static VkExportSemaphoreCreateInfo export_semaphore_create_info;
		static VkSemaphoreTypeCreateInfo   semaphore_type_create_info;
		static VkSemaphoreCreateInfo       semaphore_create_info;

	public:
		static constexpr std::string_view HOST_MEMORY_EXTENSION_NAME    = VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME;
		static constexpr std::string_view HOST_SEMAPHORE_EXTENSION_NAME = VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME;

		static VkFormat GetVkFormat(ExternalHandle::ImageFormat format);
		static ExternalHandle::ImageFormat GetImageFormat(VkFormat vk_format);

		static bool LoadVulkanHandleExtensions(VkInstance instance);
		static bool LoadCompatibleSemaphorePropsInfo(VkPhysicalDevice physical_device);

		using SEMAPHORE_GET_INFO_T = VkSemaphoreGetFdInfoKHR;
		static SEMAPHORE_GET_INFO_T  CreateSemaphoreGetInfoKHR(VkExternalSemaphoreHandleTypeFlagBits compatible_type);

		using MEMORY_GET_INFO_T = VkMemoryGetFdInfoKHR;
		static MEMORY_GET_INFO_T CreateMemoryGetInfoKHR(VkDeviceMemory memory);
		static void GetMemoryKHR(VkDevice device, const MEMORY_GET_INFO_T *info, ExternalHandle::TYPE *memory);

		static constexpr auto EXTERNAL_MEMORY_HANDLE_TYPE  = VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT;

		static constexpr VkSemaphoreCreateInfo &ExternalSemaphoreCreateInfo()
		{	return ExternalHandleVk::semaphore_create_info;	}

		static VkSemaphore CreateExternalSemaphore(VkDevice device);

		static ExternalHandle::TYPE GetSemaphoreKHR(VkDevice device, VkSemaphore semaphore);

		using IMPORT_MEMORY_INFO_KHR_T = VkImportMemoryFdInfoKHR;
		static IMPORT_MEMORY_INFO_KHR_T CreateImportMemoryInfoKHR(ExternalHandle::TYPE handle);

//		using IMPORT_SEMAPHORE_INFO_KHR_T = VkImportSemaphoreFdInfoKHR;
//		static IMPORT_SEMAPHORE_INFO_KHR_T CreateImportSemaphoreInfoKHR(ExternalHandle::TYPE handle);

		static VkSemaphore CreateImportSemaphoreKHR(VkDevice device, ExternalHandle::TYPE handle);

	private:

};

#endif //EXTERNAL_HANDLE_VK_H
