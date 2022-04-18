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
	// Check that command is locked by requester before proceeding
	if(auto cmd_lock = bipc::scoped_lock(this->_lock_data->cmd_data.cmd_request, bipc::try_to_lock); !!cmd_lock)
	{
		constexpr char ret_val = -1;
		this->_lock_data->cmd_data.cmd_processed = ret_val;
		return ret_val;
	}

	// Lock map memory access
	bipc::scoped_lock lock(this->_lock_data->map_access, bipc::try_to_lock);
	if(!lock)
	{
		lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time));

		if(!lock)
		{
			constexpr char ret_val = -2;
			this->_lock_data->cmd_data.cmd_processed = ret_val;
			return ret_val;
		}
	}

	// Process cmd
	char ret_val;
	switch(this->_lock_data->cmd_data.cmd_type)
	{
		case IPC_CMD_HANDLE_REQUEST:
			ret_val = this->ProcessHandleRequestCmd(this->_lock_data->cmd_data);
			break;

		case IPC_CMD_NAME_CHANGE:
			ret_val = this->ProcessNameChangeCmd(this->_lock_data->cmd_data);
			break;

		default:
			ret_val = -3;
	}

	this->_lock_data->cmd_data.cmd_processed = ret_val;
	return ret_val;
}

char IpcMemoryProcessorVk::ProcessNameChangeCmd(IpcCmdData &ipc_cmd)
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
	{
		// If name doesn't exist, create new data
		if(auto old_img_it = this->_image_data.find(ipc_cmd.image_name_new); old_img_it != this->_image_data.end())
			this->_image_data.erase(old_img_it);

		this->_image_data.emplace(ipc_cmd.image_name_new,
		                          this->_vk_data.CreateImage(ipc_cmd.imge_width, ipc_cmd.imge_height, ExternalHandleVk::GetVkFormat(ipc_cmd.image_format)));
	}

	return 1;
}

char IpcMemoryProcessorVk::ProcessHandleRequestCmd(IpcCmdData &ipc_cmd)
{
	auto map_data = this->_image_map->find(ipc_cmd.image_name_new);
	if(map_data == this->_image_map->end())
		return -4;

	auto img_data = this->_image_data.find(ipc_cmd.image_name_new);
	if(img_data == this->_image_data.end())
		return -5;

	map_data->second.shared_image_info = img_data->second.ExportImageInfo();
	return 1;
}
