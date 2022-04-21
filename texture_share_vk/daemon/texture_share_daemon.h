#ifndef TEXTURE_SHARE_DAEMON_H
#define TEXTURE_SHARE_DAEMON_H

#include "texture_share_vk/daemon/ipc_memory_processor_vk.h"
#include "texture_share_vk/platform/daemon_comm.h"

class TextureShareDaemon
{
		static constexpr uint64_t DEFAULT_WAIT_TIME_MICRO_S = 1000;
	public:
		TextureShareDaemon(const std::string &ipc_cmd_memory_segment = IpcMemoryProcessorVk::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                   const std::string &ipc_map_memory_segment = IpcMemoryProcessorVk::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~TextureShareDaemon() = default;

		void Initialize();

		int Loop();
		int Loop(volatile bool &run);

		int Cleanup();

	private:
		DaemonComm::LockFile _lock_file;

		IpcMemoryProcessorVk _vk_memory;
};

#endif // TEXTURE_SHARE_DAEMON_H
