#ifndef TSV_DAEMON_COMM_H
#define TSV_DAEMON_COMM_H

#include "texture_share_vk/platform/platform.h"
#include "texture_share_vk/platform/config.h"

#include <string>
#include <filesystem>


class DaemonComm
{
		struct FileDesc
		{
			FileDesc(int fd = -1);
			~FileDesc();

			FileDesc(const FileDesc &) = delete;
			FileDesc &operator=(const FileDesc &) = delete;

			FileDesc(FileDesc &&);
			FileDesc &operator=(FileDesc &&);

			constexpr operator const int&() const
			{	return this->_fd;	}

			private:
			    int _fd = -1;
		};

	public:
		struct LockFile
		{
			LockFile() = default;
			LockFile(const std::string &file);

			static bool IsFileLocked(const std::string &file);

			private:
			    FileDesc _fd;

				static int CreateLockFile(const std::string &file);
		};

		static constexpr uint64_t DEFAULT_CMD_WAIT_TIME = 1*1000*1000;//500*1000;

		static void Daemonize(const std::string &ipc_cmd_memory_segment,
		                      const std::string &ipc_map_memory_segment,
		                      uint64_t wait_time_micro_s/* = DEFAULT_CMD_WAIT_TIME*/);

		static void SendHandles(ExternalHandle::ShareHandles &&handles, const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
		static ExternalHandle::ShareHandles RecvHandles(const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		static LockFile CreateLockFile(const std::string &lock_file);
		static bool CheckLockFile(const std::string &lock_file);

	private:
		struct NamedSock
		    : public FileDesc
		{
			NamedSock(const std::filesystem::path &socket_path, int fd);
			~NamedSock();

			private:
			    std::filesystem::path _socket_path;
		};

		static int CreateNamedUnixSocket(const std::filesystem::path &socket_path, FileDesc &sock_fd);
		static int AcceptNamedUnixSocket(const FileDesc &sock_fd, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
		static int ConnectNamedUnixSocket(const std::filesystem::path &socket_path, FileDesc &sock_fd, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
};

#endif //TSV_DAEMON_COMM_H
