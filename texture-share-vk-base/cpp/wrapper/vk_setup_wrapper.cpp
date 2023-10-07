#include "vk_setup_wrapper.h"
#include <memory>

std::unique_ptr<VkSetupWrapper> vk_setup_new()
{
	return std::make_unique<VkSetupWrapper>();
}
