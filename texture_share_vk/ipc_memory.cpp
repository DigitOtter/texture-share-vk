#include "texture_share_vk/ipc_memory.h"

#include <chrono>
#include <thread>

namespace bipc = boost::interprocess;


IpcMemory::IpcData::IpcData(IpcData &&other)
    : map_access(),
      cmd_data(std::move(other.cmd_data))
{}

IpcMemory::IpcData &IpcMemory::IpcData::operator=(IpcData &&other)
{
	this->cmd_data = std::move(other.cmd_data);

	return *this;
}

IpcMemory::IpcMemory(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
    : _lock_memory_segment_name(ipc_cmd_memory_segment),
      _map_memory_segment_name(ipc_map_memory_segment)
{}

IpcMemory::~IpcMemory()
{
	if(this->_lock_data)
		this->_lock_data->map_access.lock();

	if(this->_image_map)
	{
		this->_map_memory_segment.destroy_ptr(this->_image_map);
		this->_image_map = nullptr;
	}

	bipc::shared_memory_object::remove(this->_map_memory_segment_name.c_str());

	if(this->_lock_data)
		this->_lock_data->map_access.unlock();

	bipc::shared_memory_object::remove(this->_lock_memory_segment_name.c_str());
}

bool IpcMemory::IsCmdRequestPresent() const
{
	// See if cmd_request is locked. If so, a command is ready for processing
	bipc::scoped_lock lock(this->_lock_data->cmd_data.cmd_request, bipc::try_to_lock);
	return !lock;
}

const IpcMemory::IpcCmdType &IpcMemory::CmdRequestType() const
{
	return this->_lock_data->cmd_data.cmd_type;
}

bool IpcMemory::SubmitWaitImageNameCmd(const std::string &image_name, const std::string &old_image_name,
                                       uint32_t image_width, uint32_t image_height, ExternalHandle::ImageFormat image_format,
                                       uint64_t micro_sec_wait_time)
{
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);

	bipc::scoped_lock lock(this->_lock_data->cmd_data.cmd_request, bipc::try_to_lock);
	if(!lock)
	{
		if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return false;
	}

	this->SetImageNameCmd(image_name, old_image_name,
	                      image_width, image_height, image_format);

	while(this->_lock_data->cmd_data.cmd_processed == 0&&
	      std::chrono::high_resolution_clock::now() < stop_time)
	{
		std::this_thread::sleep_for(std::chrono::microseconds(1000));
	}

	return this->_lock_data->cmd_data.cmd_processed > 0;
}

ExternalHandle::SharedImageInfo IpcMemory::SubmitWaitExternalHandleCmd(const std::string &image_name, uint64_t micro_sec_wait_time)
{
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);

	bipc::scoped_lock lock(this->_lock_data->cmd_data.cmd_request, bipc::try_to_lock);
	if(!lock)
	{
		//if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
		if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return ExternalHandle::SharedImageInfo();
	}

	this->SetHandleRequestCmd(image_name);

	while(this->_lock_data->cmd_data.cmd_processed == 0 &&
	      std::chrono::high_resolution_clock::now() < stop_time)
	{
		std::this_thread::sleep_for(std::chrono::microseconds(1000));
	}

	if(this->_lock_data->cmd_data.cmd_processed <= 0)
		return ExternalHandle::SharedImageInfo();

	bipc::sharable_lock<bipc::interprocess_sharable_mutex> map_lock(this->_lock_data->map_access, bipc::try_to_lock);
	if(!map_lock)
	{
		//if(!map_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
		if(!map_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return ExternalHandle::SharedImageInfo();
	}

	const auto image_data_it = this->_image_map->find(reinterpret_cast<const IMAGE_NAME_T&>(*image_name.c_str()));
	if(image_data_it == this->_image_map->end())
		return ExternalHandle::SharedImageInfo();

	return std::move(image_data_it->second.shared_image_info);
}

void IpcMemory::SetImageNameCmd(const std::string &image_name, const std::string &old_image_name, uint32_t image_width, uint32_t image_height, ExternalHandle::ImageFormat image_format)
{
	strncpy(this->_lock_data->cmd_data.image_name_new.data(), image_name.c_str(), sizeof(IMAGE_NAME_T));
	this->_lock_data->cmd_data.image_name_new.back() = '\0';

	strncpy(this->_lock_data->cmd_data.image_name_old.data(), old_image_name.c_str(), sizeof(IMAGE_NAME_T));
	this->_lock_data->cmd_data.image_name_old.back() = '\0';

	this->_lock_data->cmd_data.imge_width = image_width;
	this->_lock_data->cmd_data.imge_height = image_height;
	this->_lock_data->cmd_data.image_format = image_format;

	this->_lock_data->cmd_data.cmd_processed = 0;
	this->_lock_data->cmd_data.cmd_type = IPC_CMD_NAME_CHANGE;
}

void IpcMemory::SetHandleRequestCmd(const std::string &image_name)
{
	strncpy(this->_lock_data->cmd_data.image_name_new.data(), image_name.c_str(), sizeof(IMAGE_NAME_T));
	this->_lock_data->cmd_data.image_name_new.back() = '\0';

	this->_lock_data->cmd_data.cmd_processed = 0;
	this->_lock_data->cmd_data.cmd_type = IPC_CMD_HANDLE_REQUEST;
}

IpcMemory::IpcCmdData::IpcCmdData(IpcCmdData &&other)
    : cmd_request(),
      cmd_type(std::move(other.cmd_type)),
      image_name_old(std::move(other.image_name_old)),
      image_name_new(std::move(other.image_name_new)),
      imge_width(std::move(other.imge_width)),
      imge_height(std::move(other.imge_height)),
      image_format(std::move(other.image_format)),
      cmd_processed(std::move(other.cmd_processed))
{}

IpcMemory::IpcCmdData &IpcMemory::IpcCmdData::operator=(IpcCmdData &&other)
{
	this->cmd_type = std::move(other.cmd_type);
	this->image_name_old = std::move(other.image_name_old);
	this->image_name_new = std::move(other.image_name_new);
	this->imge_width = std::move(other.imge_width);
	this->imge_height = std::move(other.imge_height);
	this->image_format = std::move(other.image_format);
	this->cmd_processed = std::move(other.cmd_processed);

	return *this;
}

IpcMemory::ImageData::ImageData(ImageData &&other)
    : shared_image_info(std::move(other.shared_image_info)),
      handle_access(),
      connected_procs_count(std::move(other.connected_procs_count))
{}

IpcMemory::ImageData &IpcMemory::ImageData::operator=(ImageData &&other)
{
	this->shared_image_info = std::move(other.shared_image_info);
	this->connected_procs_count = std::move(other.connected_procs_count);

	return *this;
}
