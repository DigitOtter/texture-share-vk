#include "vk_program.h"

#include "VkBootstrap.h"

#include <iostream>
#include <SDL_vulkan.h>


#define VK_CHECK(x)                                                 \
	do                                                              \
    {                                                               \
	    VkResult err = x;                                           \
	    if (err)                                                    \
        {                                                           \
	        std::cout <<"Detected Vulkan error: " << err << std::endl; \
	        abort();                                                \
	    }                                                           \
	} while (0);

VkProgram::~VkProgram()
{
	this->Cleanup();
}

void VkProgram::Init()
{
	SDL_Init(SDL_INIT_VIDEO);

	SDL_WindowFlags window_flags = (SDL_WindowFlags)(SDL_WINDOW_VULKAN);

	this->_window = SDL_CreateWindow(
	    "Vulkan Share Test",
	    SDL_WINDOWPOS_UNDEFINED,
	    SDL_WINDOWPOS_UNDEFINED,
	    this->_window_extent.width,
	    this->_window_extent.height,
	    window_flags
	);

	//load the core Vulkan structures
	this->VulkanInit();

	this->VkInitSwapchain();
	this->VkInitCommands();
	this->VkInitDefaultRenderpass();
	this->VkInitFramebuffers();
	this->VkInitSyncStructures();

	this->_is_initialized = true;
}

void VkProgram::VulkanInit()
{
	vkb::InstanceBuilder builder;

	//make the Vulkan instance, with basic debug features
	auto inst_ret = builder.set_app_name("Vulkan Texture Share Test")
	        .request_validation_layers(true)
	        .require_api_version(1, 1, 0)
	        .use_default_debug_messenger()
	        .build();

	vkb::Instance vkb_inst = inst_ret.value();

	//store the instance
	this->_instance = vkb_inst.instance;
	//store the debug messenger
	this->_debug_messenger = vkb_inst.debug_messenger;

	// get the surface of the window we opened with SDL
	SDL_Vulkan_CreateSurface(this->_window, this->_instance, &this->_surface);

	//use vkbootstrap to select a GPU.
	//We want a GPU that can write to the SDL surface and supports Vulkan 1.1
	vkb::PhysicalDeviceSelector selector{ vkb_inst };
	vkb::PhysicalDevice physical_device = selector
	    .set_minimum_version(1, 1)
	    .set_surface(this->_surface)
	    .select()
	    .value();

	//create the final Vulkan device
	vkb::DeviceBuilder device_builder{ physical_device };

	vkb::Device vkb_device = device_builder.build().value();

	// Get the VkDevice handle used in the rest of a Vulkan application
	this->_device = vkb_device.device;
	this->_chosen_gpu = physical_device.physical_device;

	// use vkbootstrap to get a Graphics queue
	this->_graphics_queue = vkb_device.get_queue(vkb::QueueType::graphics).value();
	this->_graphics_queue_family = vkb_device.get_queue_index(vkb::QueueType::graphics).value();
}

void VkProgram::VkInitSwapchain()
{
	vkb::SwapchainBuilder swapchain_builder{ this->_chosen_gpu, this->_device, this->_surface };

	vkb::Swapchain vkb_swapchain = swapchain_builder
	    .use_default_format_selection()
	    //use vsync present mode
	    .set_desired_present_mode(VK_PRESENT_MODE_FIFO_KHR)
	    .set_desired_extent(this->_window_extent.width, this->_window_extent.height)
	    .build()
	    .value();

	//store swapchain and its related images
	this->_swapchain = vkb_swapchain.swapchain;
	this->_swapchain_images = vkb_swapchain.get_images().value();
	this->_swapchain_image_views = vkb_swapchain.get_image_views().value();

	this->_swapchain_image_format = vkb_swapchain.image_format;
}

void VkProgram::VkInitCommands()
{
	//create a command pool for commands submitted to the graphics queue.
	VkCommandPoolCreateInfo command_pool_info = {};
	command_pool_info.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
	command_pool_info.pNext = nullptr;

	//the command pool will be one that can submit graphics commands
	command_pool_info.queueFamilyIndex = this->_graphics_queue_family;
	//we also want the pool to allow for resetting of individual command buffers
	command_pool_info.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;

	VK_CHECK(vkCreateCommandPool(this->_device, &command_pool_info, nullptr, &this->_command_pool));

	//allocate the default command buffer that we will use for rendering
	VkCommandBufferAllocateInfo cmd_alloc_info = {};
	cmd_alloc_info.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
	cmd_alloc_info.pNext = nullptr;

	//commands will be made from our _commandPool
	cmd_alloc_info.commandPool = this->_command_pool;
	//we will allocate 1 command buffer
	cmd_alloc_info.commandBufferCount = 1;
	// command level is Primary
	cmd_alloc_info.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;


	VK_CHECK(vkAllocateCommandBuffers(this->_device, &cmd_alloc_info, &this->_main_command_buffer));
}

void VkProgram::VkInitDefaultRenderpass()
{
	// the renderpass will use this color attachment.
	VkAttachmentDescription color_attachment = {};
	//the attachment will have the format needed by the swapchain
	color_attachment.format = this->_swapchain_image_format;
	//1 sample, we won't be doing MSAA
	color_attachment.samples = VK_SAMPLE_COUNT_1_BIT;
	// we Clear when this attachment is loaded
	color_attachment.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
	// we keep the attachment stored when the renderpass ends
	color_attachment.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
	//we don't care about stencil
	color_attachment.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
	color_attachment.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;

	//we don't know or care about the starting layout of the attachment
	color_attachment.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

	//after the renderpass ends, the image has to be on a layout ready for display
	color_attachment.finalLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;

	VkAttachmentReference color_attachment_ref = {};
	//attachment number will index into the pAttachments array in the parent renderpass itself
	color_attachment_ref.attachment = 0;
	color_attachment_ref.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

	//we are going to create 1 subpass, which is the minimum you can do
	VkSubpassDescription subpass = {};
	subpass.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
	subpass.colorAttachmentCount = 1;
	subpass.pColorAttachments = &color_attachment_ref;

	VkRenderPassCreateInfo render_pass_info = {};
	render_pass_info.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;

	//connect the color attachment to the info
	render_pass_info.attachmentCount = 1;
	render_pass_info.pAttachments = &color_attachment;
	//connect the subpass to the info
	render_pass_info.subpassCount = 1;
	render_pass_info.pSubpasses = &subpass;

	VK_CHECK(vkCreateRenderPass(this->_device, &render_pass_info, nullptr, &this->_render_pass));
}

void VkProgram::VkInitFramebuffers()
{
	//create the framebuffers for the swapchain images. This will connect the render-pass to the images for rendering
	VkFramebufferCreateInfo fb_info = {};
	fb_info.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
	fb_info.pNext = nullptr;

	fb_info.renderPass = this->_render_pass;
	fb_info.attachmentCount = 1;
	fb_info.width = this->_window_extent.width;
	fb_info.height = this->_window_extent.height;
	fb_info.layers = 1;

	//grab how many images we have in the swapchain
	const uint32_t swapchain_imagecount = this->_swapchain_images.size();
	_framebuffers = std::vector<VkFramebuffer>(swapchain_imagecount);

	//create framebuffers for each of the swapchain image views
	for (size_t i = 0; i < swapchain_imagecount; i++) {

		fb_info.pAttachments = &this->_swapchain_image_views[i];
		VK_CHECK(vkCreateFramebuffer(this->_device, &fb_info, nullptr, &this->_framebuffers[i]));
	}
}

void VkProgram::VkInitSyncStructures()
{
	//create synchronization structures
	VkFenceCreateInfo fence_create_info = {};
	fence_create_info.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
	fence_create_info.pNext = nullptr;

	// we want to create the fence with the Create Signaled flag, so we can wait on it before using it on a GPU command (for the first frame)
	fence_create_info.flags = VK_FENCE_CREATE_SIGNALED_BIT;

	VK_CHECK(vkCreateFence(this->_device, &fence_create_info, nullptr, &this->_render_fence));

	//for the semaphores we don't need any flags
	VkSemaphoreCreateInfo semaphore_create_info = {};
	semaphore_create_info.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
	semaphore_create_info.pNext = nullptr;
	semaphore_create_info.flags = 0;

	VK_CHECK(vkCreateSemaphore(this->_device, &semaphore_create_info, nullptr, &this->_present_semaphore));
	VK_CHECK(vkCreateSemaphore(this->_device, &semaphore_create_info, nullptr, &this->_render_semaphore));
}


void VkProgram::Draw()
{
	//wait until the GPU has finished rendering the last frame. Timeout of 1 second
	VK_CHECK(vkWaitForFences(this->_device, 1, &this->_render_fence, true, 1000000000));
	VK_CHECK(vkResetFences(this->_device, 1, &this->_render_fence));

	//request image from the swapchain, one second timeout
	uint32_t swapchain_image_index;
	VK_CHECK(vkAcquireNextImageKHR(this->_device, this->_swapchain, 1000000000, this->_present_semaphore, nullptr, &swapchain_image_index));


	//now that we are sure that the commands finished executing, we can safely reset the command buffer to begin recording again.
	VK_CHECK(vkResetCommandBuffer(this->_main_command_buffer, 0));

	//naming it cmd for shorter writing
	VkCommandBuffer cmd = this->_main_command_buffer;

	//begin the command buffer recording. We will use this command buffer exactly once, so we want to let Vulkan know that
	VkCommandBufferBeginInfo cmd_begin_info = {};
	cmd_begin_info.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
	cmd_begin_info.pNext = nullptr;

	cmd_begin_info.pInheritanceInfo = nullptr;
	cmd_begin_info.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;

	VK_CHECK(vkBeginCommandBuffer(cmd, &cmd_begin_info));

	//make a clear-color from frame number. This will flash with a 120*pi frame period.
	VkClearValue clear_value;
	float flash = abs(sin(this->_frame_number / 120.f));
	clear_value.color = { { 0.0f, 0.0f, flash, 1.0f } };

	//start the main renderpass.
	//We will use the clear color from above, and the framebuffer of the index the swapchain gave us
	VkRenderPassBeginInfo rp_info = {};
	rp_info.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
	rp_info.pNext = nullptr;

	rp_info.renderPass = this->_render_pass;
	rp_info.renderArea.offset.x = 0;
	rp_info.renderArea.offset.y = 0;
	rp_info.renderArea.extent = this->_window_extent;
	rp_info.framebuffer = this->_framebuffers[swapchain_image_index];

	//connect clear values
	rp_info.clearValueCount = 1;
	rp_info.pClearValues = &clear_value;

	vkCmdBeginRenderPass(cmd, &rp_info, VK_SUBPASS_CONTENTS_INLINE);

	//finalize the render pass
	vkCmdEndRenderPass(cmd);
	//finalize the command buffer (we can no longer add commands, but it can now be executed)
	VK_CHECK(vkEndCommandBuffer(cmd));

	//prepare the submission to the queue.
	//we want to wait on the _presentSemaphore, as that semaphore is signaled when the swapchain is ready
	//we will signal the _renderSemaphore, to signal that rendering has finished

	VkSubmitInfo submit = {};
	submit.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
	submit.pNext = nullptr;

	VkPipelineStageFlags wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;

	submit.pWaitDstStageMask = &wait_stage;

	submit.waitSemaphoreCount = 1;
	submit.pWaitSemaphores = &this->_present_semaphore;

	submit.signalSemaphoreCount = 1;
	submit.pSignalSemaphores = &this->_render_semaphore;

	submit.commandBufferCount = 1;
	submit.pCommandBuffers = &cmd;

	//submit command buffer to the queue and execute it.
	// _renderFence will now block until the graphic commands finish execution
	VK_CHECK(vkQueueSubmit(this->_graphics_queue, 1, &submit, this->_render_fence));

	// this will put the image we just rendered into the visible window.
	// we want to wait on the _renderSemaphore for that,
	// as it's necessary that drawing commands have finished before the image is displayed to the user
	VkPresentInfoKHR present_info = {};
	present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
	present_info.pNext = nullptr;

	present_info.pSwapchains = &_swapchain;
	present_info.swapchainCount = 1;

	present_info.pWaitSemaphores = &this->_render_semaphore;
	present_info.waitSemaphoreCount = 1;

	present_info.pImageIndices = &swapchain_image_index;

	VK_CHECK(vkQueuePresentKHR(this->_graphics_queue, &present_info));

	//increase the number of frames drawn
	this->_frame_number++;
}

void VkProgram::Cleanup()
{
	if(this->_is_initialized)
	{
		//make sure the gpu has stopped doing its things
		vkDeviceWaitIdle(this->_device);

		vkDestroyCommandPool(this->_device, this->_command_pool, nullptr);

		//destroy sync objects
		vkDestroyFence(this->_device, this->_render_fence, nullptr);
		vkDestroySemaphore(this->_device, this->_render_semaphore, nullptr);
		vkDestroySemaphore(this->_device, this->_present_semaphore, nullptr);

		vkDestroySwapchainKHR(this->_device, this->_swapchain, nullptr);

		vkDestroyRenderPass(this->_device, this->_render_pass, nullptr);

		//destroy swapchain resources
		for (size_t i = 0; i < this->_framebuffers.size(); i++)
		{
			vkDestroyFramebuffer(this->_device, this->_framebuffers[i], nullptr);

			vkDestroyImageView(this->_device, this->_swapchain_image_views[i], nullptr);
		}

		vkDestroySurfaceKHR(this->_instance, this->_surface, nullptr);

		vkDestroyDevice(this->_device, nullptr);
		vkb::destroy_debug_utils_messenger(this->_instance, this->_debug_messenger);
		vkDestroyInstance(this->_instance, nullptr);

		SDL_DestroyWindow(this->_window);

		this->_is_initialized = false;
	}
}
