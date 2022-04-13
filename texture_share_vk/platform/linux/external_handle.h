#ifndef EXTERNAL_HANDLE_H
#define EXTERNAL_HANDLE_H

class ExternalHandle
{
	public:
		using TYPE = int;
		static constexpr TYPE INVALID_VALUE = -1;

		struct ShareHandles
		{
			ExternalHandle::TYPE memory    {ExternalHandle::INVALID_VALUE};
			ExternalHandle::TYPE ext_read  {ExternalHandle::INVALID_VALUE};
			ExternalHandle::TYPE ext_write {ExternalHandle::INVALID_VALUE};
		};

	private:
};

#endif //EXTERNAL_HANDLE_H
