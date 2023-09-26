#include "texture_share_vk/platform/config.h"
#include "texture_share_vk/platform/daemon_comm.h"

#include <fcntl.h>
#include <future>

int main(int /*argc*/, char * /*argv*/[])
{
	std::exception_ptr teptr_send = nullptr;
	std::exception_ptr teptr_recv = nullptr;

	const std::filesystem::path sock_path = std::filesystem::path(TSV_DAEMON_SOCKET_DIR) / "test.sock";

	DaemonComm::FileDesc fd1 = open("test1", O_CREAT | O_RDWR | O_TRUNC, 0666);
	DaemonComm::FileDesc fd2 = open("test2", O_CREAT | O_RDWR | O_TRUNC, 0666);
	DaemonComm::FileDesc fd3 = open("test3", O_CREAT | O_RDWR | O_TRUNC, 0666);

	DaemonComm::FileDesc rfd1;
	DaemonComm::FileDesc rfd2;
	DaemonComm::FileDesc rfd3;

	const auto send_fcn = [&]() {
		try
		{
			DaemonComm comm;
			comm.CreateLockFile(TSV_DAEMON_LOCK_FILE);

			ExternalHandle::ShareHandles handles;
			handles.ext_read  = fd1;
			handles.ext_write = fd2;
			handles.memory    = fd3;

			comm.SendHandles(std::move(handles), sock_path);
		}
		catch(...)
		{
			teptr_send = std::current_exception();
		}
	};

	const auto recv_fcn = [&]() {
		try
		{
			DaemonComm comm;
			auto handles = comm.RecvHandles(sock_path);

			rfd1 = handles.ext_read;
			rfd2 = handles.ext_write;
			rfd3 = handles.memory;
		}
		catch(...)
		{
			teptr_recv = std::current_exception();
		}
	};

	std::future<void> thread2 = std::async(std::launch::async, recv_fcn);
	sleep(1);
	std::future<void> thread1 = std::async(std::launch::async, send_fcn);

	thread1.wait();
	if(teptr_send)
		std::rethrow_exception(teptr_send);

	thread2.wait();
	if(teptr_recv)
		std::rethrow_exception(teptr_recv);

	if(rfd1 == ExternalHandle::INVALID_VALUE || rfd2 == ExternalHandle::INVALID_VALUE ||
	   rfd3 == ExternalHandle::INVALID_VALUE)
		return -1;

	return 0;
}
