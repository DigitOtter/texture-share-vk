#ifndef IPC_MEMORY_H
#define IPC_MEMORY_H

#include "texture_share_vk/platform/platform.h"

#include <boost/interprocess/allocators/allocator.hpp>
#include <boost/interprocess/containers/map.hpp>
#include <boost/interprocess/managed_shared_memory.hpp>
#include <boost/interprocess/sync/interprocess_mutex.hpp>
#include <boost/interprocess/sync/interprocess_sharable_mutex.hpp>
#include <boost/interprocess/sync/scoped_lock.hpp>
#include <boost/interprocess/sync/sharable_lock.hpp>
#include <cstring>
#include <utility>


class IpcMemory
{
	protected:
		static constexpr uint64_t DEFAULT_CMD_WAIT_TIME = 500*1000;

	public:
		static constexpr std::string_view DEFAULT_IPC_CMD_MEMORY_NAME = "SharedTextureCmdMemory";
		static constexpr std::string_view DEFAULT_IPC_MAP_MEMORY_NAME = "SharedTextureMapMemory";

		using IMAGE_NAME_T = std::array<char, 1024>;

		enum IpcCmdType
		{
			IPC_CMD_NAME_CHANGE,
			IPC_CMD_HANDLE_REQUEST,
		};

		struct IpcCmdData
		{
			boost::interprocess::interprocess_mutex cmd_request;
			IpcCmdType cmd_type;
			IMAGE_NAME_T image_name_old{"\0"};
			IMAGE_NAME_T image_name_new{"\0"};
			uint32_t imge_width = 0;
			uint32_t imge_height = 0;
			ExternalHandle::ImageFormat image_format = ExternalHandle::IMAGE_FORMAT_MAX_ENUM;
			volatile char cmd_processed = 0;

			IpcCmdData() = default;

			IpcCmdData(IpcCmdData &&);
			IpcCmdData &operator=(IpcCmdData &&);
		};

		struct IpcData
		{
			// Manage access to image map
			boost::interprocess::interprocess_sharable_mutex map_access;

			IpcCmdData cmd_data;

			IpcData() = default;

			IpcData(IpcData &&);
			IpcData &operator=(IpcData &&);
		};

		struct ImageData
		{
			ExternalHandle::SharedImageInfo shared_image_info{};
			boost::interprocess::interprocess_mutex handle_access;
			uint32_t connected_procs_count = 0;

			ImageData() = default;

			ImageData(ImageData &&);
			ImageData &operator=(ImageData &&);
		};

		IpcMemory(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		          const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~IpcMemory();

		bool IsCmdRequestPresent() const;
		const IpcCmdType &CmdRequestType() const;

		bool SubmitWaitImageNameCmd(const std::string &image_name, const std::string &old_image_name = "",
		                            uint32_t image_width = 0, uint32_t image_height = 0, ExternalHandle::ImageFormat image_format = ExternalHandle::IMAGE_FORMAT_MAX_ENUM,
		                            uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		ExternalHandle::SharedImageInfo SubmitWaitExternalHandleCmd(const std::string &image_name, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

	protected:
		struct ImageNameCompare
		{
				bool operator() (const IMAGE_NAME_T &x, const IMAGE_NAME_T &y) const
				{
					return strcmp(x.data(), y.data()) >= 0;
				}
		};

		using map_value_t = std::pair<const IMAGE_NAME_T, ImageData>;

		using shmem_allocator_t = boost::interprocess::allocator<map_value_t, boost::interprocess::managed_shared_memory::segment_manager>;
		using shmem_map_t = boost::interprocess::map<const IMAGE_NAME_T, ImageData, ImageNameCompare, shmem_allocator_t>;

		std::string _lock_memory_segment_name = DEFAULT_IPC_CMD_MEMORY_NAME.data();
		std::string _map_memory_segment_name = DEFAULT_IPC_MAP_MEMORY_NAME.data();

		boost::interprocess::managed_shared_memory _lock_memory_segment =
		        boost::interprocess::managed_shared_memory(boost::interprocess::create_only, this->_lock_memory_segment_name.c_str(), sizeof(IpcData));

		IpcData *_lock_data = this->_lock_memory_segment.construct<IpcData>(boost::interprocess::unique_instance)(IpcData());

		boost::interprocess::managed_shared_memory _map_memory_segment =
		        boost::interprocess::managed_shared_memory(boost::interprocess::create_only, this->_map_memory_segment_name.c_str(), 65536);

		shmem_allocator_t _map_allocator = shmem_allocator_t(this->_map_memory_segment.get_segment_manager());

		shmem_map_t *_image_map = this->_map_memory_segment.construct<shmem_map_t>(boost::interprocess::unique_instance)(ImageNameCompare(), this->_map_allocator);

	private:
		void SetImageNameCmd(const std::string &image_name, const std::string &old_image_name,
		                     uint32_t image_width, uint32_t image_height, ExternalHandle::ImageFormat image_format);
		void SetHandleRequestCmd(const std::string &image_name);
};

#endif // IPC_MEMORY_H
