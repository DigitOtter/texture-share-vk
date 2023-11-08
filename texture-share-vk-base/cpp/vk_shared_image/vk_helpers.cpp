#include "vk_helpers.h"

#include "VkBootstrap.h"

#include "texture_share_vk/platform/external_handle_vk.h"

// #include "texture_share_vk/platform/platform_vk.h"

VkHelpers::TextureShareVkStruct VkHelpers::CreateTextureShareVkInstance()
{
	// make the Vulkan instance, with basic debug features
	vkb::InstanceBuilder builder;
	auto inst_ret = builder
	                    .set_app_name("Vulkan Texture Share Test")
	                    //.request_validation_layers(true)
	                    .request_validation_layers(false)
	                    .require_api_version(1, 2, 0)
	                    .set_headless(true)
	                    //.use_default_debug_messenger()
	                    //.enable_extension(VK_EXT_DEBUG_REPORT_EXTENSION_NAME)

	                    .enable_extension(VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2_EXTENSION_NAME)

	                    .enable_extension(VK_KHR_EXTERNAL_SEMAPHORE_CAPABILITIES_EXTENSION_NAME)
	                    .enable_extension(VK_KHR_EXTERNAL_MEMORY_CAPABILITIES_EXTENSION_NAME)

	                    .build();

	vkb::Instance vkb_inst = inst_ret.value();

	VkPhysicalDeviceVulkan12Features features{VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_2_FEATURES, nullptr};
	features.timelineSemaphore = true;

	// use vkbootstrap to select a GPU.
	// We want a GPU that supports Vulkan 1.2
	vkb::PhysicalDeviceSelector selector{vkb_inst};
	vkb::PhysicalDevice physical_device =
		selector
			.set_minimum_version(1, 2)

			.add_required_extension(VK_KHR_EXTERNAL_SEMAPHORE_EXTENSION_NAME)
			.add_required_extension(VK_KHR_EXTERNAL_MEMORY_EXTENSION_NAME)

			.add_required_extension(VK_KHR_TIMELINE_SEMAPHORE_EXTENSION_NAME)

			.add_required_extension(ExternalHandleVk::HOST_SEMAPHORE_EXTENSION_NAME.data())
			.add_required_extension(ExternalHandleVk::HOST_MEMORY_EXTENSION_NAME.data())

			.set_required_features_12(features)

			.select()
			.value();

	// create the final Vulkan device
	vkb::DeviceBuilder device_builder{physical_device};

	vkb::Device vkb_device = device_builder.build().value();

	TextureShareVkStruct vk_struct;
	vk_struct.instance        = vkb_inst.instance;
	vk_struct.debug_messenger = vkb_inst.debug_messenger;
	vk_struct.physical_device = vkb_device.physical_device;
	vk_struct.device          = vkb_device.device;

	// use vkbootstrap to get a Graphics queue
	vk_struct.graphics_queue       = vkb_device.get_queue(vkb::QueueType::graphics).value();
	vk_struct.graphics_queue_index = vkb_device.get_queue_index(vkb::QueueType::graphics).value();

	return vk_struct;
}

void VkHelpers::CleanupTextureShareVkInstance(TextureShareVkStruct vk_struct, bool destroy_instance,
                                              bool destroy_device)
{
	// make sure the gpu has stopped doing its things
	if(vk_struct.device != VK_NULL_HANDLE)
		vkDeviceWaitIdle(vk_struct.device);

	if(destroy_device && vk_struct.device != VK_NULL_HANDLE)
	{
		vkDestroyDevice(vk_struct.device, nullptr);
		vk_struct.device = VK_NULL_HANDLE;
	}

	if(vk_struct.debug_messenger != VK_NULL_HANDLE)
	{
		vkb::destroy_debug_utils_messenger(vk_struct.instance, vk_struct.debug_messenger);
		vk_struct.debug_messenger = VK_NULL_HANDLE;
	}

	if(destroy_instance && vk_struct.instance != VK_NULL_HANDLE)
	{
		vkDestroyInstance(vk_struct.instance, nullptr);
	}
	vk_struct.instance = VK_NULL_HANDLE;
}

VkCommandPool VkHelpers::CreateCommandPool(VkDevice device, uint32_t queue_family_index)
{
	VkCommandPool command_pool;

	// create a command pool for commands submitted to the graphics queue.
	VkCommandPoolCreateInfo command_pool_info = {};
	command_pool_info.sType                   = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
	command_pool_info.pNext                   = nullptr;

	// the command pool will be one that can submit graphics commands
	command_pool_info.queueFamilyIndex = queue_family_index;
	// we also want the pool to allow for resetting of individual command buffers
	command_pool_info.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;

	VK_CHECK(vkCreateCommandPool(device, &command_pool_info, nullptr, &command_pool));

	return command_pool;
}

void VkHelpers::CleanupCommandPool(VkDevice device, VkCommandPool command_pool)
{
	vkDestroyCommandPool(device, command_pool, nullptr);
}

VkCommandBuffer VkHelpers::CreateCommandBuffer(VkDevice device, VkCommandPool command_pool)
{
	// allocate the default command buffer that we will use for rendering
	VkCommandBufferAllocateInfo cmd_alloc_info = {};
	cmd_alloc_info.sType                       = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
	cmd_alloc_info.pNext                       = nullptr;

	// commands will be made from our _commandPool
	cmd_alloc_info.commandPool = command_pool;
	// we will allocate 1 command buffer
	cmd_alloc_info.commandBufferCount = 1;
	// command level is Primary
	cmd_alloc_info.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;


	VkCommandBuffer command_buffer;
	VK_CHECK(vkAllocateCommandBuffers(device, &cmd_alloc_info, &command_buffer));

	return command_buffer;
}

void VkHelpers::CleanupCommandBuffer(VkDevice device, VkCommandPool command_pool, VkCommandBuffer command_buffer)
{
	vkFreeCommandBuffers(device, command_pool, 1, &command_buffer);
}

VkCommandBufferBeginInfo VkHelpers::CommandBufferBeginInfoSingleUse()
{
	VkCommandBufferBeginInfo cmd_begin_info{VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO};
	cmd_begin_info.pNext = nullptr;

	cmd_begin_info.pInheritanceInfo = nullptr;
	cmd_begin_info.flags            = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;

	return cmd_begin_info;
}

void VkHelpers::ImmediateSubmit(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer,
                                const std::function<void(VkCommandBuffer)> &f, VkSemaphore signal_semaphore)
{
	return VkHelpers::ImmediateSubmit(device, queue, command_buffer, f, VK_NULL_HANDLE, 0, &signal_semaphore,
	                                  signal_semaphore != VK_NULL_HANDLE ? 1 : 0);
}

void VkHelpers::ImmediateSubmit(VkDevice device, VkQueue queue, VkCommandBuffer command_buffer,
                                const std::function<void(VkCommandBuffer)> &f, VkSemaphore *wait_semaphores,
                                uint32_t wait_semaphore_count, VkSemaphore *signal_semaphores,
                                uint32_t signal_semaphore_count)
{
	VkCommandBufferBeginInfo cmd_begin_info = CommandBufferBeginInfoSingleUse();

	VK_CHECK(vkBeginCommandBuffer(command_buffer, &cmd_begin_info));

	f(command_buffer);

	if(command_buffer == VK_NULL_HANDLE)
		return;

	VK_CHECK(vkEndCommandBuffer(command_buffer));

	VkSubmitInfo submit_info{VK_STRUCTURE_TYPE_SUBMIT_INFO, nullptr};
	submit_info.commandBufferCount   = 1;
	submit_info.pCommandBuffers      = &command_buffer;
	submit_info.pSignalSemaphores    = signal_semaphores;
	submit_info.signalSemaphoreCount = signal_semaphore_count;
	submit_info.pWaitSemaphores      = wait_semaphores;
	submit_info.waitSemaphoreCount   = wait_semaphore_count;

	// Create fence to ensure that the command buffer has finished executing
	VkFenceCreateInfo fence_info{VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, nullptr};
	fence_info.flags = 0;

	VkFence fence;
	VK_CHECK(vkCreateFence(device, &fence_info, nullptr, &fence));

	// Submit to the queue
	VK_CHECK(vkQueueSubmit(queue, 1, &submit_info, fence));
	// Wait for the fence to signal that command buffer has finished executing
	VK_CHECK(vkWaitForFences(device, 1, &fence, VK_TRUE, VkHelpers::DEFAULT_FENCE_TIMEOUT));

	vkDestroyFence(device, fence, nullptr);
}

void VkHelpers::CmdPipelineMemoryBarrierColorImage(VkCommandBuffer command_buffer, VkImage image,
                                                   VkImageLayout old_layout, VkImageLayout new_layout,
                                                   VkAccessFlagBits src_access_mask, VkAccessFlagBits dst_access_mask,
                                                   VkPipelineStageFlagBits pipeline_stage_flags)
{
	VkImageMemoryBarrier image_memory_barrier  = {VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER};
	image_memory_barrier.srcQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;
	image_memory_barrier.dstQueueFamilyIndex   = VK_QUEUE_FAMILY_IGNORED;
	image_memory_barrier.image                 = image;
	image_memory_barrier.srcAccessMask         = src_access_mask;
	image_memory_barrier.dstAccessMask         = dst_access_mask;
	image_memory_barrier.oldLayout             = old_layout;
	image_memory_barrier.newLayout             = new_layout;
	VkImageSubresourceRange &subresource_range = image_memory_barrier.subresourceRange;
	subresource_range.aspectMask               = VK_IMAGE_ASPECT_COLOR_BIT;
	subresource_range.levelCount               = 1;
	subresource_range.layerCount               = 1;

	vkCmdPipelineBarrier(command_buffer, pipeline_stage_flags, VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT, 0, 0,
	                     nullptr, 0, nullptr, 1, &image_memory_barrier);
}

uint32_t VkHelpers::GetMemoryType(VkPhysicalDevice physical_device, uint32_t bits, VkMemoryPropertyFlags properties,
                                  VkBool32 *memory_type_found)
{
	VkPhysicalDeviceMemoryProperties memory_properties;
	vkGetPhysicalDeviceMemoryProperties(physical_device, &memory_properties);

	for(uint32_t i = 0; i < memory_properties.memoryTypeCount; i++)
	{
		if((bits & 1) == 1)
		{
			if((memory_properties.memoryTypes[i].propertyFlags & properties) == properties)
			{
				if(memory_type_found)
				{
					*memory_type_found = true;
				}
				return i;
			}
		}
		bits >>= 1;
	}

	if(memory_type_found)
	{
		*memory_type_found = false;
		return 0;
	}
	else
	{
		throw std::runtime_error("Could not find a matching memory type");
	}
}
