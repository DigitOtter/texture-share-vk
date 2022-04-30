#ifndef TSV_DAEMON_COMM_H
#define TSV_DAEMON_COMM_H

#include "texture_share_vk/platform/platform.h"
#include "texture_share_vk/platform/config.h"

#include <string>
#include <filesystem>
#include <Windows.h>


class DaemonComm
{
		struct FileDesc
		{
			FileDesc(HANDLE fd = nullptr);
			~FileDesc();

			FileDesc(const FileDesc &) = delete;
			FileDesc &operator=(const FileDesc &) = delete;

			FileDesc(FileDesc &&);
			FileDesc &operator=(FileDesc &&);

			constexpr operator const HANDLE&() const
			{	return this->_fd;	}

			private:
			    HANDLE _fd = nullptr;
		};

	public:
		struct LockFile
		{
			LockFile() = default;
			LockFile(const std::string &file, bool create_directory = false);

			static bool IsFileLocked(const std::string &file);

			private:
			    FileDesc _fd;

				static HANDLE CreateLockFile(const std::string &file, bool create_directory);
		};

		static constexpr uint64_t DEFAULT_CMD_WAIT_TIME = 1*1000*1000;//500*1000;

		using PROC_T = HANDLE;
		static constexpr PROC_T INVALID_PROC = nullptr;

		static PROC_T GetProcId();
		static bool IsProcRunning(PROC_T pid);

		static void Daemonize(const std::string &ipc_cmd_memory_segment,
		                      const std::string &ipc_map_memory_segment,
		                      uint64_t wait_time_micro_s/* = DEFAULT_CMD_WAIT_TIME*/);

		static void SendHandles(ExternalHandle::ShareHandles &&handles, const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
		static void RecvHandles(const std::filesystem::path &socket_path, ExternalHandle::ShareHandles &handles, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		static LockFile CreateLockFile(const std::string &lock_file);
		static bool CheckLockFile(const std::string &lock_file);
};

#endif //TSV_DAEMON_COMM_H
