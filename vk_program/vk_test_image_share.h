#ifndef VK_TEST_IMAGE_SHARE_H
#define VK_TEST_IMAGE_SHARE_H

// Most of the code is taken from https://vkguide.dev

#include "texture_share_vk/texture_share_vk_client.h"

#include <SDL.h>
#include <vector>


class VkTestImageShare
{
		static constexpr int WIN_WIDTH = 800;
		static constexpr int WIN_HEIGHT = 640;

	public:
		VkTestImageShare() = default;
		~VkTestImageShare();

		void Init();

		void VulkanInit();
		void VkInitSwapchain();
		void VkInitCommands();
		void VkInitDefaultRenderpass();
		void VkInitFramebuffers();
		void VkInitSyncStructures();

		void VkInitExternals();
		void VkInitSharedImage();

		void SetExternalHandle(SharedImageHandleVk *shared_image_handle);

		void Draw();

		void DrawSharedImage(VkImage swapchain_image, VkImageLayout image_layout, VkFence fence);

		void Cleanup();
		void VkCleanup();

	private:
		SDL_Window *_window = nullptr;
		const VkExtent2D _window_extent = {WIN_WIDTH, WIN_HEIGHT};

		size_t _frame_number = 0;

		bool _is_initialized = false;

		const std::string _shared_image_name = "test_vk";

		VkInstance _instance;
		VkDebugUtilsMessengerEXT _debug_messenger; // Vulkan debug output handle
		VkPhysicalDevice _chosen_gpu; // GPU chosen as the default device
		VkDevice _device; // Vulkan device for commands
		VkSurfaceKHR _surface; // Vulkan window surface

		VkSwapchainKHR _swapchain;
		VkFormat _swapchain_image_format;
		std::vector<VkImage> _swapchain_images;
		std::vector<VkImageView> _swapchain_image_views;

		VkQueue _graphics_queue;
		uint32_t _graphics_queue_family;
		VkCommandPool _command_pool;
		VkCommandBuffer _main_command_buffer;

		VkRenderPass _render_pass;
		std::vector<VkFramebuffer> _framebuffers;

		VkSemaphore _present_semaphore;
		VkSemaphore _render_semaphore;
		VkFence _render_fence;

		TextureShareVkClient _shared_image_client;
};

#endif //VK_TEST_IMAGE_SHARE_H
