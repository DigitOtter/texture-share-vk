#ifndef EXTERNAL_HANDLE_H
#define EXTERNAL_HANDLE_H

#include <cstdint>
#include <cstring>
#include <unistd.h>

class ExternalHandle
{
	public:
	using TYPE                          = int;
	static constexpr TYPE INVALID_VALUE = -1;

	struct ShareHandles
	{
		ExternalHandle::TYPE memory = ExternalHandle::INVALID_VALUE;
		// ExternalHandle::TYPE ext_read{ExternalHandle::INVALID_VALUE};
		// ExternalHandle::TYPE ext_write{ExternalHandle::INVALID_VALUE};

		// Ensure only move operations are allowed. Takes care of ownership transfers
		ShareHandles(const ShareHandles &)            = delete;
		ShareHandles &operator=(const ShareHandles &) = delete;

		ShareHandles(ShareHandles &&other);
		ShareHandles &operator=(ShareHandles &&other);

		ShareHandles() = default;
		~ShareHandles();
	};


	private:
};

#endif // EXTERNAL_HANDLE_H
