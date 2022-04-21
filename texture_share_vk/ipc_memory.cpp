#include "texture_share_vk/ipc_memory.h"

#include "texture_share_vk/platform/daemon_comm.h"

#include <chrono>
#include <thread>
#include <boost/interprocess/segment_manager.hpp>

namespace bipc = boost::interprocess;


IpcMemory::IpcData::IpcData(IpcData &&other)
    : map_access(),
      cmd_request_access(),
      next_cmd_num(std::move(other.next_cmd_num)),
      processed_cmd_num(std::move(other.processed_cmd_num))
{}

IpcMemory::IpcData &IpcMemory::IpcData::operator=(IpcData &&other)
{
	this->next_cmd_num = std::move(other.next_cmd_num);
	this->processed_cmd_num = std::move(other.processed_cmd_num);

	return *this;
}

IpcMemory::ImageData::ImageData(ImageData &&other)
    : shared_image_info(std::move(other.shared_image_info)),
      socket_filename(std::move(other.socket_filename)),
      handle_access(),
      connected_procs_count(std::move(other.connected_procs_count))
{}

IpcMemory::ImageData &IpcMemory::ImageData::operator=(ImageData &&other)
{
	this->shared_image_info = std::move(other.shared_image_info);
	this->socket_filename = std::move(other.socket_filename);
	this->connected_procs_count = std::move(other.connected_procs_count);

	return *this;
}

IpcMemory::IpcMemory(const std::string &ipc_cmd_memory_segment,
                     const std::string &ipc_map_memory_segment)
    : IpcMemory(bipc::create_only,
                ipc_cmd_memory_segment,
                ipc_map_memory_segment)
{}

IpcMemory::IpcMemory(bipc::create_only_t,
                     const std::string &ipc_cmd_memory_segment,
                     const std::string &ipc_map_memory_segment)
    : _lock_memory_segment_name(ipc_cmd_memory_segment),
      _map_memory_segment_name(ipc_map_memory_segment),
      _owns_segment(true),
      _shmem_remover(ipc_cmd_memory_segment, ipc_map_memory_segment),
      _cmd_memory_segment(bipc::create_only, ipc_cmd_memory_segment.c_str(),
                          100, IPC_QUEUE_MSG_SIZE),
      _map_memory_segment(bipc::managed_shared_memory(bipc::create_only,
                                                      this->_map_memory_segment_name.c_str(),
                                                      sizeof(IpcData) + sizeof(shmem_map_t) +
                                                      10*sizeof(map_value_t) + 1024)),
      _map_allocator(shmem_allocator_t(this->_map_memory_segment.get_segment_manager())),
      _lock_data(this->_map_memory_segment.find_or_construct<IpcData>(bipc::unique_instance)(IpcData())),
      _image_map(this->_map_memory_segment.find_or_construct<shmem_map_t>(bipc::unique_instance)(ImageNameCompare(),
                                                                                                 this->_map_allocator))
{}

IpcMemory::IpcMemory(bipc::open_or_create_t,
                     const std::string &ipc_cmd_memory_segment,
                     const std::string &ipc_map_memory_segment)
    : _lock_memory_segment_name(ipc_cmd_memory_segment),
      _map_memory_segment_name(ipc_map_memory_segment),
      _owns_segment(false),
      _shmem_remover("", ""),
      _cmd_memory_segment(bipc::open_or_create, ipc_cmd_memory_segment.c_str(),
                          100, IPC_QUEUE_MSG_SIZE),
      _map_memory_segment(bipc::managed_shared_memory(bipc::open_or_create,
                                                      this->_map_memory_segment_name.c_str(),
                                                      sizeof(IpcData) + sizeof(shmem_map_t) +
                                                      10*sizeof(map_value_t) + 1024)),
      _map_allocator(shmem_allocator_t(this->_map_memory_segment.get_segment_manager())),
      _lock_data(this->_map_memory_segment.find_or_construct<IpcData>(bipc::unique_instance)(IpcData())),
      _image_map(this->_map_memory_segment.find_or_construct<shmem_map_t>(bipc::unique_instance)(ImageNameCompare(),
                                                                                                 this->_map_allocator))
{}

IpcMemory::~IpcMemory()
{
	if(this->_lock_data)
		this->_lock_data->map_access.lock();

	if(this->_image_map && this->_owns_segment)
	{
		this->_map_memory_segment.destroy_ptr(this->_image_map);
	}

	this->_image_map = nullptr;

	if(this->_owns_segment)
		bipc::shared_memory_object::remove(this->_map_memory_segment_name.c_str());

	if(this->_lock_data)
		this->_lock_data->map_access.unlock();

	this->_lock_data = nullptr;

	if(this->_owns_segment)
		bipc::shared_memory_object::remove(this->_lock_memory_segment_name.c_str());
}

bool IpcMemory::SubmitWaitImageInitCmd(const std::string &image_name,
                                       uint32_t image_width, uint32_t image_height, ExternalHandle::ImageFormat image_format,
                                       uint64_t micro_sec_wait_time)
{
	uint32_t cmd_req_num;

	// Lock cmd_request_access until command is sent
	{
		bipc::scoped_lock lock(this->_lock_data->cmd_request_access, bipc::try_to_lock);
		if(!lock)
		{
			if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
				return false;
		}

		cmd_req_num = this->_lock_data->next_cmd_num++;

		IpcCmdImageInit cmd{IPC_CMD_IMAGE_INIT, cmd_req_num};

		if(image_name.length() >= cmd.image_name.size() + 1)
			throw std::runtime_error("Image name '" + image_name + "' too large");

		strcpy(cmd.image_name.data(), image_name.c_str());
		cmd.imge_width = image_width;
		cmd.imge_height = image_height;
		cmd.image_format = image_format;

		this->_cmd_memory_segment.send(&cmd, sizeof(cmd), IPC_QUEUE_MSG_PRIORITY_DEFAULT);
	}

	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	while(cmd_req_num > this->_lock_data->processed_cmd_num &&
	      std::chrono::high_resolution_clock::now() < stop_time)
	{
		std::this_thread::sleep_for(std::chrono::microseconds(1000));
	}

	return cmd_req_num <= this->_lock_data->processed_cmd_num;
}

bool IpcMemory::SubmitWaitImageRenameCmd(const std::string &image_name, const std::string &old_image_name,
                                         uint64_t micro_sec_wait_time)
{
	uint32_t cmd_req_num;

	// Lock cmd_request_access until command is sent
	{
		bipc::scoped_lock lock(this->_lock_data->cmd_request_access, bipc::try_to_lock);
		if(!lock)
		{
			if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
				return false;
		}

		cmd_req_num = this->_lock_data->next_cmd_num++;

		IpcCmdRename cmd{IPC_CMD_RENAME, cmd_req_num};

		if(image_name.length() >= cmd.image_name_new.size() + 1)
			throw std::runtime_error("Image name '" + image_name + "' too large");
		if(old_image_name.length() >= cmd.image_name_old.size() + 1)
			throw std::runtime_error("Image name '" + old_image_name + "' too large");

		strcpy(cmd.image_name_new.data(), image_name.c_str());
		strcpy(cmd.image_name_old.data(), old_image_name.c_str());
		this->_cmd_memory_segment.send(&cmd, sizeof(cmd), IPC_QUEUE_MSG_PRIORITY_DEFAULT);
	}

	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	while(cmd_req_num > this->_lock_data->processed_cmd_num &&
	      std::chrono::high_resolution_clock::now() < stop_time)
	{
		std::this_thread::sleep_for(std::chrono::microseconds(1000));
	}

	return cmd_req_num <= this->_lock_data->processed_cmd_num;
}

ExternalHandle::SharedImageInfo IpcMemory::SubmitWaitExternalHandleCmd(const std::string &image_name, uint64_t micro_sec_wait_time)
{
	// Lock cmd_request_access until command is sent and handle is retrieved
	bipc::scoped_lock lock(this->_lock_data->cmd_request_access, bipc::try_to_lock);
	if(!lock)
	{
		if(!lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return ExternalHandle::SharedImageInfo{};
	}

	const uint32_t cmd_req_num = this->_lock_data->next_cmd_num++;

	IpcCmdRequestImageHandles cmd{IPC_CMD_HANDLE_REQUEST, cmd_req_num};

	if(image_name.length() >= cmd.image_name.size() + 1)
		throw std::runtime_error("Image name '" + image_name + "' too large");

	strcpy(cmd.image_name.data(), image_name.c_str());
	this->_cmd_memory_segment.send(&cmd, sizeof(cmd), IPC_QUEUE_MSG_PRIORITY_DEFAULT);

	auto img_data_it = this->_image_map->find(cmd.image_name);
	if(img_data_it == this->_image_map->end())
		return ExternalHandle::SharedImageInfo{};

	// Check if socket name set up by server
	const auto stop_time = std::chrono::high_resolution_clock::now() + std::chrono::microseconds(micro_sec_wait_time);
	while(img_data_it->second.socket_filename.front() == '\0' &&
	      std::chrono::high_resolution_clock::now() < stop_time)
	{
		std::this_thread::sleep_for(std::chrono::microseconds(micro_sec_wait_time)/10);
	}

	if(img_data_it->second.socket_filename.front() == '\0')
		return ExternalHandle::SharedImageInfo{};

	ExternalHandle::SharedImageInfo img_info = std::move(img_data_it->second.shared_image_info);
	img_info.handles = DaemonComm::RecvHandles(img_data_it->second.socket_filename.data());

	img_data_it->second.socket_filename.front() = '\0';

	return img_info;
}

IpcMemory::ImageData *IpcMemory::GetImageData(const std::string &image_name, uint64_t micro_sec_wait_time) const
{
	bipc::sharable_lock<bipc::interprocess_sharable_mutex> map_lock(this->_lock_data->map_access, bipc::try_to_lock);
	if(!map_lock)
	{
		if(!map_lock.try_lock_for(std::chrono::microseconds(micro_sec_wait_time)))
			return nullptr;
	}

	const IMAGE_NAME_T &data = reinterpret_cast<const IMAGE_NAME_T &>(*image_name.c_str());
	if(auto img_data_it = this->_image_map->find(data); img_data_it != this->_image_map->end())
		return &(img_data_it->second);

	return nullptr;
}
