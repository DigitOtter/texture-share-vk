#include "texture_share_vk_setup.hpp"
#include <utility>

TextureShareVkSetup::TextureShareVkSetup()
	: _setup(vk_setup_new())
{}

TextureShareVkSetup::~TextureShareVkSetup()
{
	vk_setup_destroy(this->_setup);
}

TextureShareVkSetup::TextureShareVkSetup(TextureShareVkSetup &&other)
	: _setup(std::move(other._setup))
{
	other._setup = nullptr;
}

TextureShareVkSetup &TextureShareVkSetup::operator=(TextureShareVkSetup &&other)
{
	this->_setup = std::move(other._setup);
	other._setup = nullptr;

	return *this;
}

void TextureShareVkSetup::initialize_vulkan()
{
	if(!this->_setup)
		return;

	return vk_setup_initialize_vulkan(this->_setup);
}

void TextureShareVkSetup::import_vulkan(VkInstance instance, VkDevice device, VkPhysicalDevice physical_device,
                                        VkQueue graphics_queue, uint32_t graphics_queue_index, bool import_only)
{
	if(!this->_setup)
		return;

	return vk_setup_import_vulkan(this->_setup, instance, device, physical_device, graphics_queue, graphics_queue_index,
	                              import_only);
}

VkSetup *TextureShareVkSetup::release()
{
	VkSetup *ret = this->_setup;
	this->_setup = nullptr;

	return ret;
}
