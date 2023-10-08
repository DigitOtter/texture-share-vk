#include "external_handle.h"

#include <utility>

using namespace opengl;

ExternalHandle::ShareHandles::ShareHandles(ShareHandles &&other)
	: memory(std::move(other.memory))
// ext_read(std::move(other.ext_read)),
// ext_write(std::move(other.ext_write))
{
	other.memory = ExternalHandle::INVALID_VALUE;
	// other.ext_write = INVALID_VALUE;
	// other.ext_read = INVALID_VALUE;
}

ExternalHandle::ShareHandles &ExternalHandle::ShareHandles::operator=(ShareHandles &&other)
{
	this->~ShareHandles();

	memcpy(this, &other, sizeof(ShareHandles));
	other.memory = ExternalHandle::INVALID_VALUE;
	// other.ext_write = INVALID_VALUE;
	// other.ext_read = INVALID_VALUE;
	return *this;
}

ExternalHandle::ShareHandles::~ShareHandles()
{
	// Close all file descriptors if ownership was not transferred to Vulkan via import
	if(memory >= 0)
	{
		close(memory);
		memory = ExternalHandle::INVALID_VALUE;
	}

	// if(ext_read >= 0)
	// {
	// 	close(ext_read);
	// 	ext_read = INVALID_VALUE;
	// }

	// if(ext_write >= 0)
	// {
	// 	close(ext_write);
	// 	ext_write = INVALID_VALUE;
	// }
}
