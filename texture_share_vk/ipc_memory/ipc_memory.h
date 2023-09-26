#ifndef IPC_MEMORY_H
#define IPC_MEMORY_H

#include "ipc_commands.h"
#include "texture_share_vk/platform/daemon_comm.h"
// #include "texture_share_vk/platform/platform.h"

#include <boost/interprocess/allocators/allocator.hpp>
#include <boost/interprocess/containers/map.hpp>
#include <boost/interprocess/ipc/message_queue.hpp>
#include <boost/interprocess/managed_shared_memory.hpp>
#include <boost/interprocess/sync/interprocess_mutex.hpp>
#include <boost/interprocess/sync/interprocess_sharable_mutex.hpp>
#include <boost/interprocess/sync/scoped_lock.hpp>
#include <boost/interprocess/sync/sharable_lock.hpp>

#include <array>
#include <cstring>
#include <utility>

class IpcMemory
{
	public:
	static constexpr uint64_t DEFAULT_CMD_WAIT_TIME                    = 100 * 1000 * 1000; // 500*1000;
	static constexpr uint64_t DAEMON_STARTUP_DEFAULT_WAIT_TIME_MICRO_S = 100 * 1000 * 1000;

	static constexpr std::string_view DEFAULT_IPC_CMD_MEMORY_NAME = "SharedTextureCmdMemory";
	static constexpr std::string_view DEFAULT_IPC_MAP_MEMORY_NAME = "SharedTextureMapMemory";

	using IMAGE_NAME_T = ipc_commands::IMAGE_NAME_T;

	struct IpcData
	{
		// Manage access to image map
		boost::interprocess::interprocess_sharable_mutex map_access;

		boost::interprocess::interprocess_mutex cmd_request_access;
		DaemonComm::PROC_T calling_pid = DaemonComm::INVALID_PROC;
		uint32_t next_cmd_num          = 1;
		uint32_t processed_cmd_num     = 0;

		IpcData() = default;

		IpcData(IpcData &&);
		IpcData &operator=(IpcData &&);
	};

	struct ImageData
	{
		ExternalHandle::SharedImageInfo shared_image_info{};
		ipc_commands::SOCKET_FILENAME_T socket_filename;

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

	static IpcMemory CreateIpcClientAndDaemon(
		const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data(),
		uint64_t wait_time_micro_s                = DAEMON_STARTUP_DEFAULT_WAIT_TIME_MICRO_S);

	bool SubmitWaitRegisterProcCmd(DaemonComm::PROC_T proc_id   = DaemonComm::GetProcId(),
	                               uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

	bool SubmitWaitImageInitCmd(const std::string &image_name, uint32_t image_width, uint32_t image_height,
	                            ExternalHandle::ImageFormat image_format, bool overwrite_existing = false,
	                            uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

	bool SubmitWaitImageRenameCmd(const std::string &image_name, const std::string &old_image_name = "",
	                              uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

	ExternalHandle::SharedImageInfo SubmitWaitExternalHandleCmd(const std::string &image_name,
	                                                            uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
	ImageData *GetImageData(const std::string &image_name, uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME) const;

	protected:
	using map_value_t = std::pair<const IMAGE_NAME_T, ImageData>;

	using shmem_allocator_t =
		boost::interprocess::allocator<map_value_t, boost::interprocess::managed_shared_memory::segment_manager>;
	using shmem_map_t =
		boost::interprocess::map<const IMAGE_NAME_T, ImageData, ipc_commands::ImageNameCompare, shmem_allocator_t>;

	std::string _lock_memory_segment_name = DEFAULT_IPC_CMD_MEMORY_NAME.data();
	std::string _map_memory_segment_name  = DEFAULT_IPC_MAP_MEMORY_NAME.data();

	bool _owns_segment = false;

	// Empty struct. Just here to remove old storage memory
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

	IpcData *_lock_data     = nullptr;
	shmem_map_t *_image_map = nullptr;
};

#endif // IPC_MEMORY_H
