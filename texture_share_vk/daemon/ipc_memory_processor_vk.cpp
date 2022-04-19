#include "texture_share_vk/daemon/ipc_memory_processor_vk.h"

#include <chrono>


namespace bipc = boost::interprocess;

IpcMemoryProcessorVk::IpcMemoryProcessorVk(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
    : IpcMemory(ipc_cmd_memory_segment, ipc_map_memory_segment)
{}

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
}

char IpcMemoryProcessorVk::ProcessCmd(uint64_t micro_sec_wait_time)
{
	// Lock map memory access
	bipc::scoped_lock lock(this->_lock_data->map_access, bipc::try_to_lock);
	if(!lock)
	{
		lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time));

		if(!lock)
		{
			constexpr char ret_val = -2;
			return ret_val;
		}
	}

	// Process cmd
	char ret_val;
	unsigned char buffer[IpcMemory::IPC_QUEUE_MSG_SIZE];
	size_t recv_size;
	unsigned int prio;

	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	if(!this->_cmd_memory_segment.timed_receive(buffer, sizeof(buffer), recv_size, prio, stop_time))
		return -3;

	uint32_t cmd_num = 0;
	switch(*reinterpret_cast<const IpcCmdType*>(buffer))
	{
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

char IpcMemoryProcessorVk::ProcessRenameCmd(const IpcCmdRename &ipc_cmd)
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

char IpcMemoryProcessorVk::ProcessImageInitCmd(const IpcCmdImageInit &ipc_cmd)
{
	if(auto old_map_it = this->_image_map->find(ipc_cmd.image_name); old_map_it != this->_image_map->end())
	{
		bipc::scoped_lock lock(old_map_it->second.handle_access);

		auto data_it = this->_image_data.find(ipc_cmd.image_name);
		if(data_it == this->_image_data.end())
			return -6;

		data_it->second = this->_vk_data.CreateImage(ipc_cmd.imge_width, ipc_cmd.imge_height, ExternalHandleVk::GetVkFormat(ipc_cmd.image_format));
	}
	else
	{
		// If name doesn't exist, create new data
		if(auto old_img_it = this->_image_data.find(ipc_cmd.image_name); old_img_it != this->_image_data.end())
			this->_image_data.erase(old_img_it);

		this->_image_data.emplace(ipc_cmd.image_name,
		                          this->_vk_data.CreateImage(ipc_cmd.imge_width, ipc_cmd.imge_height, ExternalHandleVk::GetVkFormat(ipc_cmd.image_format)));

		this->_image_map->emplace(ipc_cmd.image_name, IpcMemory::ImageData());
	}

	return 1;
}

char IpcMemoryProcessorVk::ProcessHandleRequestCmd(const IpcCmdRequestImageHandles &ipc_cmd)
{
	auto map_data = this->_image_map->find(ipc_cmd.image_name);
	if(map_data == this->_image_map->end())
		return -7;

	auto img_data = this->_image_data.find(ipc_cmd.image_name);
	if(img_data == this->_image_data.end())
		return -8;

	map_data->second.shared_image_info = img_data->second.ExportImageInfo();
	return 1;
}
