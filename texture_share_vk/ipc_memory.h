#ifndef IPC_MEMORY_H
#define IPC_MEMORY_H

#include "texture_share_vk/platform/daemon_comm.h"
#include "texture_share_vk/platform/platform.h"

#include <boost/interprocess/allocators/allocator.hpp>
#include <boost/interprocess/containers/map.hpp>
#include <boost/interprocess/ipc/message_queue.hpp>
#include <boost/interprocess/managed_shared_memory.hpp>
#include <boost/interprocess/sync/interprocess_mutex.hpp>
#include <boost/interprocess/sync/interprocess_sharable_mutex.hpp>
#include <boost/interprocess/sync/scoped_lock.hpp>
#include <boost/interprocess/sync/sharable_lock.hpp>
#include <cstring>
#include <utility>


class IpcMemory
{
	public:
		static constexpr uint64_t DEFAULT_CMD_WAIT_TIME = 1*1000*1000; //500*1000;
		static constexpr uint64_t DAEMON_STARTUP_DEFAULT_WAIT_TIME_MICRO_S = 1*1000*1000;

		static constexpr std::string_view DEFAULT_IPC_CMD_MEMORY_NAME = "SharedTextureCmdMemory";
		static constexpr std::string_view DEFAULT_IPC_MAP_MEMORY_NAME = "SharedTextureMapMemory";

		using IMAGE_NAME_T = std::array<char, 1024>;
		using SOCKET_FILENAME_T = std::array<char, 1024>;

		enum IpcCmdType
		{
			IPC_CMD_REGISTER_PROC,
			IPC_CMD_IMAGE_INIT,
			IPC_CMD_RENAME,
			IPC_CMD_HANDLE_REQUEST,
		};

		struct IpcCmdRegisterProc
		{
			IpcCmdType cmd_type;
			uint32_t cmd_num;
			DaemonComm::PROC_T proc_id = DaemonComm::INVALID_PROC;
		};

		struct IpcCmdImageInit
		{
			IpcCmdType cmd_type;
			uint32_t cmd_num;
			IMAGE_NAME_T image_name{"\0"};
			uint32_t imge_width = 0;
			uint32_t imge_height = 0;
			ExternalHandle::ImageFormat image_format = ExternalHandle::IMAGE_FORMAT_MAX_ENUM;
			bool overwrite_existing = false;
		};

		struct IpcCmdRename
		{
			IpcCmdType cmd_type;
			uint32_t cmd_num;
			IMAGE_NAME_T image_name_new{"\0"};
			IMAGE_NAME_T image_name_old{"\0"};
		};

		struct IpcCmdRequestImageHandles
		{
			IpcCmdType cmd_type;
			uint32_t cmd_num;
			IMAGE_NAME_T image_name{"\0"};
		};

		struct IpcData
		{
			// Manage access to image map
			boost::interprocess::interprocess_sharable_mutex map_access;

			boost::interprocess::interprocess_mutex cmd_request_access;
			DaemonComm::PROC_T calling_pid = DaemonComm::INVALID_PROC;
			uint32_t next_cmd_num = 1;
			uint32_t processed_cmd_num = 0;

			IpcData() = default;

			IpcData(IpcData &&);
			IpcData &operator=(IpcData &&);
		};

		struct ImageData
		{
			ExternalHandle::SharedImageInfo shared_image_info{};
			SOCKET_FILENAME_T socket_filename;

			boost::interprocess::interprocess_sharable_mutex handle_access;
			uint32_t connected_procs_count = 0;

			ImageData() = default;

			ImageData(ImageData &&);
			ImageData &operator=(ImageData &&);
		};

		IpcMemory(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		          const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		IpcMemory(boost::interprocess::create_only_t,
		          const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		          const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		IpcMemory(boost::interprocess::open_or_create_t,
		          const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		          const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~IpcMemory();

		static IpcMemory CreateIpcClientAndDaemon(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                                          const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data(),
		                                          uint64_t wait_time_micro_s = DAEMON_STARTUP_DEFAULT_WAIT_TIME_MICRO_S);

		bool SubmitWaitRegisterProcCmd(DaemonComm::PROC_T proc_id = DaemonComm::GetProcId(),
		                               uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		bool SubmitWaitImageInitCmd(const std::string &image_name,
		                            uint32_t image_width, uint32_t image_height, ExternalHandle::ImageFormat image_format,
		                            bool overwrite_existing = false,
		                            uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		bool SubmitWaitImageRenameCmd(const std::string &image_name, const std::string &old_image_name = "",
		                              uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

		ExternalHandle::SharedImageInfo SubmitWaitExternalHandleCmd(const std::string &image_name, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
		ImageData *GetImageData(const std::string &image_name, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME) const;

	protected:
		template<class T, class ...Ts>
		static constexpr size_t MultiMax()
		{
			if constexpr (sizeof...(Ts) > 0)
			{	return std::max(sizeof(T), MultiMax<Ts...>());	}
			else
			{	return sizeof(T);	}
		}

		static constexpr size_t IPC_QUEUE_MSG_SIZE = std::max(std::max(sizeof(IpcCmdImageInit),
		                                                               sizeof(IpcCmdRename)),
		                                                               sizeof(IpcCmdRequestImageHandles));

		static constexpr unsigned int IPC_QUEUE_MSG_PRIORITY_DEFAULT = 50;

		struct ImageNameCompare
		{
				bool operator() (const IMAGE_NAME_T &x, const IMAGE_NAME_T &y) const
				{	return strcmp(x.data(), y.data()) < 0;	}
		};

		using map_value_t = std::pair<const IMAGE_NAME_T, ImageData>;

		using shmem_allocator_t = boost::interprocess::allocator<map_value_t, boost::interprocess::managed_shared_memory::segment_manager>;
		using shmem_map_t = boost::interprocess::map<const IMAGE_NAME_T, ImageData, ImageNameCompare, shmem_allocator_t>;

		std::string _lock_memory_segment_name = DEFAULT_IPC_CMD_MEMORY_NAME.data();
		std::string _map_memory_segment_name = DEFAULT_IPC_MAP_MEMORY_NAME.data();

		bool _owns_segment = false;

		struct shmem_remover
		{
			shmem_remover(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
			              const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data())
			{
				boost::interprocess::shared_memory_object::remove(ipc_cmd_memory_segment.c_str());
				boost::interprocess::shared_memory_object::remove(ipc_map_memory_segment.c_str());
			}
		} _shmem_remover;

		boost::interprocess::message_queue _cmd_memory_segment;


		boost::interprocess::managed_shared_memory _map_memory_segment;
		shmem_allocator_t _map_allocator;

		IpcData *_lock_data = nullptr;
		shmem_map_t *_image_map = nullptr;
};

#endif // IPC_MEMORY_H
