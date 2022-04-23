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

#endif //EXTERNAL_HANDLE_H
