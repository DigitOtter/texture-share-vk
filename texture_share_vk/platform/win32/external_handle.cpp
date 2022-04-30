#include "external_handle.h"

#include <handleapi.h>
#include <utility>


ExternalHandle::ShareHandles::ShareHandles(ShareHandles &&other)
    : memory(std::move(other.memory)),
      ext_read(std::move(other.ext_read)),
      ext_write(std::move(other.ext_write))
{
	other.memory = INVALID_VALUE;
	other.ext_write = INVALID_VALUE;
	other.ext_read = INVALID_VALUE;
}

ExternalHandle::ShareHandles &ExternalHandle::ShareHandles::operator=(ShareHandles &&other)
{
	this->~ShareHandles();

	memcpy(this, &other, sizeof(ShareHandles));
	other.memory = INVALID_VALUE;
	other.ext_write = INVALID_VALUE;
	other.ext_read = INVALID_VALUE;
	return *this;
}

ExternalHandle::ShareHandles::~ShareHandles()
{
	// Close all file descriptors if ownership was not transferred to Vulkan via import
	if(memory != INVALID_VALUE)
	{
		CloseHandle(memory);
		memory = INVALID_VALUE;
	}

	if(ext_read != INVALID_VALUE)
	{
		CloseHandle(ext_read);
		ext_read = INVALID_VALUE;
	}

	if(ext_write != INVALID_VALUE)
	{
		CloseHandle(ext_write);
		ext_write = INVALID_VALUE;
	}
}
