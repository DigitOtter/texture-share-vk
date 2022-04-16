#ifndef EXTERNAL_HANDLE_H
#define EXTERNAL_HANDLE_H

#include <cstdint>
#include <cstring>
#include <unistd.h>


class ExternalHandle
{
	public:
		using TYPE = int;
		static constexpr TYPE INVALID_VALUE = -1;

		enum ImageFormat
		{
			R8G8B8A8,
			IMAGE_FORMAT_MAX_ENUM
		};

		struct ShareHandles
		{
			ExternalHandle::TYPE memory    {ExternalHandle::INVALID_VALUE};
			ExternalHandle::TYPE ext_read  {ExternalHandle::INVALID_VALUE};
			ExternalHandle::TYPE ext_write {ExternalHandle::INVALID_VALUE};

			// Ensure only move operations are allowed. Takes care of ownership transfers
			ShareHandles(const ShareHandles &) = delete;
			ShareHandles &operator=(const ShareHandles &) = delete;

			ShareHandles(ShareHandles &&other);
			ShareHandles &operator=(ShareHandles &&other);

			ShareHandles() = default;
			~ShareHandles();
		};

		struct SharedImageInfo
		{
			ShareHandles handles;
			uint32_t width, height;
			ImageFormat format;
		};

	private:
};

inline ExternalHandle::ShareHandles::ShareHandles(ShareHandles &&other)
{
	memcpy(this, &other, sizeof(ShareHandles));
	other.memory = INVALID_VALUE;
	other.ext_write = INVALID_VALUE;
	other.ext_read = INVALID_VALUE;
}

inline ExternalHandle::ShareHandles &ExternalHandle::ShareHandles::operator=(ShareHandles &&other)
{
	memcpy(this, &other, sizeof(ShareHandles));
	other.memory = INVALID_VALUE;
	other.ext_write = INVALID_VALUE;
	other.ext_read = INVALID_VALUE;
	return *this;
}

inline ExternalHandle::ShareHandles::~ShareHandles()
{
	// Close all file descriptors if ownership was not transferred to Vulkan via import
	if(memory >= 0)
	{
		close(memory);
		memory = INVALID_VALUE;
	}

	if(ext_read >= 0)
	{
		close(ext_read);
		ext_read = INVALID_VALUE;
	}

	if(ext_write >= 0)
	{
		close(ext_write);
		ext_write = INVALID_VALUE;
	}
}



#endif //EXTERNAL_HANDLE_H
