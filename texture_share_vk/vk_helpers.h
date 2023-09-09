#ifndef VK_HELPERS_H
#define VK_HELPERS_H

#include <functional>
#include <iostream>

#include <vulkan/vulkan.hpp>


#define VK_CHECK(x)                                                     \
	do                                                                  \
	{                                                                   \
		VkResult err = x;                                               \
		if(err)                                                         \
		{                                                               \
			std::cerr << "Detected Vulkan error: " << err << std::endl; \
			abort();                                                    \
		}                                                               \
	}                                                                   \
	while(0);

class VkHelpers
{
	public:
	static constexpr uint64_t DEFAULT_FENCE_TIMEOUT = 100000000000;

	struct TextureShareVkStruct
	{
		VkInstance instance;
		VkDebugUtilsMessengerEXT debug_messenger;
		VkDevice device;
		VkPhysicalDevice physical_device;
		VkQueue graphics_queue;
		uint32_t graphics_queue_index;
	};

	static TextureShareVkStruct CreateTextureShareVkInstance();
	static void CleanupTextureShareVkInstance(TextureShareVkStruct vk_struct, bool destroy_instance = true,
	                                          bool destroy_device = true);

	static VkCommandPool CreateCommandPool(VkDevice device, uint32_t queue_family_index);
	static void CleanupCommandPool(VkDevice device, VkCommandPool command_pool);

	static VkCommandBuffer CreateCommandBuffer(VkDevice device, VkCommandPool command_pool);
	static void CleanupCommandBuffer(VkDevice device, VkCommandPool command_pool, VkCommandBuffer command_buffer);

	static VkImageMemoryBarrier CreateImageMemoryBarrier();

	static void ImmediateSubmit(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer,
	                            const std::function<void(VkCommandBuffer command_buffer)> &f,
	                            VkSemaphore signal_semaphore);

	static void ImmediateSubmit(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer,
	                            const std::function<void(VkCommandBuffer command_buffer)> &f,
	                            VkSemaphore *wait_semaphores, uint32_t wait_semaphore_count,
	                            VkSemaphore *signal_semaphores, uint32_t signal_semaphore_count);

	static void CmdClearColorImage(VkCommandBuffer command_buffer, VkImage image, const VkClearColorValue &color_value,
	                               VkImageLayout image_layout);
	static void CmdPipelineMemoryBarrierColorImage(
		VkCommandBuffer command_buffer, VkImage image, VkImageLayout old_layout, VkImageLayout new_layout,
		VkAccessFlagBits src_access_mask, VkAccessFlagBits dst_access_mask,
		VkPipelineStageFlagBits pipeline_stage_flags = VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT);

	static uint32_t GetMemoryType(VkPhysicalDevice physical_device, uint32_t bits, VkMemoryPropertyFlags properties,
	                              VkBool32 *memory_type_found = nullptr);
};

#endif // VK_HELPERS_H
