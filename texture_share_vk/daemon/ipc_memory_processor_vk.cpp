#include "texture_share_vk/daemon/ipc_memory_processor_vk.h"

#include "texture_share_vk/platform/config.h"

#include <chrono>
#include <csignal>
#include <filesystem>


namespace bipc = boost::interprocess;
using namespace ipc_commands;

IpcMemoryProcessorVk::IpcMemoryProcessorVk(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
    : IpcMemory(ipc_cmd_memory_segment, ipc_map_memory_segment)
{
	// Make sure no sockets are in directory
	for(const std::filesystem::directory_entry &dir_entry : std::filesystem::recursive_directory_iterator(TSV_DAEMON_SOCKET_DIR))
	{
		if(dir_entry.exists() && dir_entry.is_socket())
			std::filesystem::remove(dir_entry.path());
	}

	// Create socket directory
	std::filesystem::create_directories(TSV_DAEMON_SOCKET_DIR);
}

IpcMemoryProcessorVk::~IpcMemoryProcessorVk()
{
	this->CleanupVulkan();
}

void IpcMemoryProcessorVk::InitializeVulkan()
{
	this->_vk_data.InitializeVulkan();
}

void IpcMemoryProcessorVk::CleanupVulkan()
{
	bipc::scoped_lock lock(this->_lock_data->map_access);

	for(auto &img_data : this->_image_data)
	{
		img_data.second.Cleanup();
	}
	this->_image_data.clear();

	this->_image_map->clear();

	this->_vk_data.CleanupVulkan();
}

char IpcMemoryProcessorVk::ProcessCmd(uint64_t micro_sec_wait_time)
{
	// Process cmd
	char ret_val;
	unsigned char buffer[IPC_QUEUE_MSG_SIZE];
	size_t recv_size;
	unsigned int prio;

	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	if(!this->_cmd_memory_segment.timed_receive(buffer, sizeof(buffer), recv_size, prio, stop_time))
		return -3;

	// Lock map memory access
	const auto tp = bipc::ipcdetail::duration_to_ustime(std::chrono::microseconds(micro_sec_wait_time));
	bipc::scoped_lock lock(this->_lock_data->map_access, tp);
	if(!lock)
	{
			constexpr char ret_val = -2;
			return ret_val;
	}

	uint32_t cmd_num = 0;
	switch(*reinterpret_cast<const IpcCmdType*>(buffer))
	{
		case IPC_CMD_REGISTER_PROC:
			ret_val = this->ProcessRegisterProcCmd(*reinterpret_cast<const IpcCmdRegisterProc*>(buffer));
			cmd_num = reinterpret_cast<const IpcCmdRegisterProc*>(buffer)->cmd_num;
			break;

		case IPC_CMD_IMAGE_INIT:
			ret_val = this->ProcessImageInitCmd(*reinterpret_cast<const IpcCmdImageInit*>(buffer));
			cmd_num = reinterpret_cast<const IpcCmdImageInit*>(buffer)->cmd_num;
			break;

		case IPC_CMD_RENAME:
			ret_val = this->ProcessRenameCmd(*reinterpret_cast<const IpcCmdRename*>(buffer));
			cmd_num = reinterpret_cast<const IpcCmdRename*>(buffer)->cmd_num;
			break;

		case IPC_CMD_HANDLE_REQUEST:
			ret_val = this->ProcessHandleRequestCmd(*reinterpret_cast<const IpcCmdRequestImageHandles*>(buffer));
			cmd_num = reinterpret_cast<const IpcCmdRequestImageHandles*>(buffer)->cmd_num;
			break;

		default:
			ret_val = -4;
	}

	if(cmd_num > 0)
		this->_lock_data->processed_cmd_num = cmd_num;

	return ret_val;
}

void IpcMemoryProcessorVk::CleanupLocks()
{
	if(this->_lock_data->calling_pid <= 0)
		return;

	bipc::scoped_lock lock(this->_lock_data->cmd_request_access, bipc::try_to_lock);

	// If cmd request is still locked by another process, check other process's status
	if(!lock)
	{
		if(!DaemonComm::IsProcRunning(this->_lock_data->calling_pid))
		{
			// Unlock if process has died
			std::cerr << "Unlocking cmd of dead process" << std::endl;
			this->_lock_data->cmd_request_access.unlock();
		}
	}
}

bool IpcMemoryProcessorVk::CheckConnectedProcs()
{
	for(auto pid_it = this->_registered_pids.begin(); pid_it != this->_registered_pids.end();)
	{
		if(!DaemonComm::IsProcRunning(*pid_it))
		{
			this->_registered_pids.erase(pid_it++);
		}
		else
			++pid_it;
	}

	return !this->_registered_pids.empty();
}

char IpcMemoryProcessorVk::ProcessRegisterProcCmd(const ipc_commands::IpcCmdRegisterProc &ipc_cmd)
{
	this->_registered_pids.emplace(ipc_cmd.proc_id);
	return 1;
}

char IpcMemoryProcessorVk::ProcessImageInitCmd(const ipc_commands::IpcCmdImageInit &ipc_cmd)
{
	if(auto old_map_it = this->_image_map->find(ipc_cmd.image_name); old_map_it != this->_image_map->end())
	{
		// TODO: Add timeout
		bipc::scoped_lock lock(old_map_it->second.handle_access);

		auto data_it = this->_image_data.find(ipc_cmd.image_name);
		if(data_it == this->_image_data.end())
			return -6;

		// Erase old image if requested
		if(ipc_cmd.overwrite_existing)
		{
			data_it->second.Cleanup();
			data_it->second =
				this->_vk_data.CreateImage(ipc_cmd.imge_width, ipc_cmd.imge_height, this->_next_image_id++,
			                               ExternalHandleVk::GetVkFormat(ipc_cmd.image_format));
		}
	}
	else
	{
		// If name doesn't exist, create new data
		if(auto old_img_it = this->_image_data.find(ipc_cmd.image_name); old_img_it != this->_image_data.end())
			this->_image_data.erase(old_img_it);

		this->_image_data.emplace(ipc_cmd.image_name,
		                          this->_vk_data.CreateImage(ipc_cmd.imge_width, ipc_cmd.imge_height,
		                                                     this->_next_image_id++,
		                                                     ExternalHandleVk::GetVkFormat(ipc_cmd.image_format)));

		this->_image_map->emplace(ipc_cmd.image_name, IpcMemory::ImageData());
	}

	return 1;
}

char IpcMemoryProcessorVk::ProcessRenameCmd(const ipc_commands::IpcCmdRename &ipc_cmd)
{
	if(auto old_data_it = this->_image_map->find(ipc_cmd.image_name_old); old_data_it != this->_image_map->end())
	{
		// If name exists, move data to new location
		auto res = this->_image_map->emplace(ipc_cmd.image_name_new, ImageData());

		res.first->second.connected_procs_count = 1;
		res.first->second.shared_image_info = std::move(old_data_it->second.shared_image_info);

		this->_image_map->erase(old_data_it);
	}
	else
		return -5;

	return 1;
}

char IpcMemoryProcessorVk::ProcessHandleRequestCmd(const ipc_commands::IpcCmdRequestImageHandles &ipc_cmd)
{
	auto map_data = this->_image_map->find(ipc_cmd.image_name);
	if(map_data == this->_image_map->end())
		return -7;

	auto img_data = this->_image_data.find(ipc_cmd.image_name);
	if(img_data == this->_image_data.end())
		return -8;

	map_data->second.socket_filename.front() = '\0';

	// Create handles and image info
	map_data->second.shared_image_info = img_data->second.ExportImageInfo();
	auto handles = std::move(map_data->second.shared_image_info.handles);

	// Share socket filename
	const std::filesystem::path sock_filename = std::filesystem::path(TSV_DAEMON_SOCKET_DIR) / ipc_cmd.image_name.data();
	strncpy(map_data->second.socket_filename.data(), sock_filename.c_str(), map_data->second.socket_filename.size());

	// Send handles
	DaemonComm::SendHandles(std::move(handles), sock_filename);

	return 1;
}
