#ifndef TEXTURE_SHARE_VK_H
#define TEXTURE_SHARE_VK_H

#include "texture_share_vk/shared_image_vk.h"

#include <vulkan/vulkan.hpp>

class TextureShareVk
{
	public:
		TextureShareVk() = default;
		~TextureShareVk() = default;

	private:

		SharedImageVk _shared_image;
};

#endif //TEXTURE_SHARE_VK_H
