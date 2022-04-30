#include "daemon_comm.h"

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/platform/config.h"

#include <csignal>
#include <iostream>
#include <thread>


DaemonComm::FileDesc::FileDesc(HANDLE fd)
    : _fd(fd)
{}

DaemonComm::FileDesc::FileDesc(FileDesc &&other)
    : _fd(std::move(other._fd))
{
	other._fd = nullptr;
}

DaemonComm::FileDesc &DaemonComm::FileDesc::operator=(FileDesc &&other)
{
	this->~FileDesc();

	this->_fd = std::move(other._fd);
	other._fd = nullptr;

	return *this;
}

DaemonComm::FileDesc::~FileDesc()
{
	if(this->_fd != nullptr)
	{
		CloseHandle(this->_fd);
		this->_fd = nullptr;
	}
}

DaemonComm::LockFile::LockFile(const std::string &file, bool create_directory)
    : _fd(CreateLockFile(file, create_directory))
{}

bool DaemonComm::LockFile::IsFileLocked(const std::string &file)
{
	try
	{
		FileDesc fd = LockFile::CreateLockFile(file, true);
	}
	catch(const std::logic_error &)
	{
		return false;
	}

	return true;
}

HANDLE DaemonComm::LockFile::CreateLockFile(const std::string &file, bool create_directory)
{
	// Create socket directory
	if(create_directory)
		std::filesystem::create_directories(TSV_DAEMON_SOCKET_DIR);

	// Try to create and open file
	std::wstring stemp = std::wstring(file.begin(), file.end());
	HANDLE fd = CreateFile2(stemp.c_str(), GENERIC_READ | GENERIC_WRITE, 0, CREATE_ALWAYS, nullptr);

	if (fd == nullptr)
		throw std::runtime_error("Failed to open lock file");

	OVERLAPPED fd_data;
	ZeroMemory(&fd_data, sizeof(fd_data));
	fd_data.hEvent = nullptr;
	fd_data.Internal = 0;
	fd_data.InternalHigh = 0;

	// To prevent race conditions, don't remove() lock on destruction
	if(!LockFileEx(fd, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY, 0, 1, 0, &fd_data))
	{
		CloseHandle(fd);
		throw std::logic_error("Failed to acquire lock");
	}

	return fd;
}

DaemonComm::PROC_T DaemonComm::GetProcId()
{
	return GetCurrentProcess();
}

bool DaemonComm::IsProcRunning(PROC_T pid)
{
	// TODO: Check if process is running
	return true;
}

void DaemonComm::Daemonize(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment, uint64_t wait_time_micro_s)
{
	// Only spawn daemon if not yet started
	if(!DaemonComm::CheckLockFile(TSV_DAEMON_LOCK_FILE))
		return;
	
	std::string cmd_line = std::string(TSV_DAEMON_PATH) + " " + ipc_cmd_memory_segment + " " + ipc_map_memory_segment;

	STARTUPINFO st_info;
	ZeroMemory(&st_info, sizeof(st_info));
	st_info.cb = sizeof(st_info);

	PROCESS_INFORMATION proc_inf;
	ZeroMemory(&proc_inf, sizeof(proc_inf));
	if(!CreateProcessA(TSV_DAEMON_PATH, cmd_line.data(), nullptr, nullptr, false, 0, nullptr, nullptr, &st_info, &proc_inf))
		throw std::runtime_error("Failed to create texture share daemon");

	// Parent process: Wait for spawn to complete
	bool daemon_running;

	// Check at least every 100ms
	//const auto interval = std::min(std::chrono::microseconds(100 * 1000), std::chrono::microseconds(wait_time_micro_s) / 10);
	std::chrono::microseconds interval;
	if (std::chrono::microseconds(100 * 1000) < std::chrono::microseconds(wait_time_micro_s) / 10)
		interval = std::chrono::microseconds(100 * 1000);
	else
		interval = std::chrono::microseconds(wait_time_micro_s) / 10;
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(wait_time_micro_s);
	do
	{
		daemon_running = !DaemonComm::CheckLockFile(TSV_DAEMON_LOCK_FILE);
		if(daemon_running)
			break;

		std::this_thread::sleep_for(interval);
	}
	while(std::chrono::high_resolution_clock::now() <= stop_time);

	if(!daemon_running)
		throw std::runtime_error("Failed to start daemon");
}

void DaemonComm::SendHandles(ExternalHandle::ShareHandles &&handles, const std::filesystem::path &socket_path, uint64_t micro_sec_wait_time)
{
	// Nothing to do in windows. Handles are global
}

void DaemonComm::RecvHandles(const std::filesystem::path &socket_path, ExternalHandle::ShareHandles &handles, uint64_t micro_sec_wait_time)
{
	// Nothing to do in windows. Handles are global
}

DaemonComm::LockFile DaemonComm::CreateLockFile(const std::string &lock_file)
{
	return LockFile(lock_file);
}

bool DaemonComm::CheckLockFile(const std::string &lock_file)
{
	return LockFile::IsFileLocked(lock_file);
}