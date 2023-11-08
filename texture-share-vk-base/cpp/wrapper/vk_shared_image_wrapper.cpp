#include "vk_shared_image_wrapper.h"

#include <memory>

std::unique_ptr<VkSharedImageWrapper> vk_shared_image_new()
{
	return std::make_unique<VkSharedImageWrapper>();
}

std::unique_ptr<ShareHandlesWrapper> vk_share_handles_new()
{
	return std::make_unique<ShareHandlesWrapper>();
}

std::unique_ptr<ShareHandlesWrapper> vk_share_handles_from_fd(int memory_fd)
{
	ExternalHandle::ShareHandles handles;
	handles.memory = memory_fd;
	return std::make_unique<ShareHandlesWrapper>(std::move(handles));
}
