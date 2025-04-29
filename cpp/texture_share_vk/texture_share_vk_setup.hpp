#pragma once

#include "texture_share_vk/texture_share_vk_base.h"

class TextureShareVkSetup
{
	public:
	TextureShareVkSetup();
	~TextureShareVkSetup();

	TextureShareVkSetup(const TextureShareVkSetup &)            = delete;
	TextureShareVkSetup &operator=(const TextureShareVkSetup &) = delete;
	TextureShareVkSetup(TextureShareVkSetup &&other);
	TextureShareVkSetup &operator=(TextureShareVkSetup &&other);

	void initialize_vulkan();
	void import_vulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device, VkQueue graphics_queue,
	                   uint32_t graphics_queue_index, bool import_only);

	VkSetup *release();

	private:
	VkSetup *_setup = nullptr;
};
