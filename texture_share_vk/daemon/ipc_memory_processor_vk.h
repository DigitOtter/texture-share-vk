#ifndef IPC_MEMORY_PROCESSOR_VK_H
#define IPC_MEMORY_PROCESSOR_VK_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/texture_share_vk.h"

#include <map>


class IpcMemoryProcessorVk
        : public IpcMemory
{
	public:
		IpcMemoryProcessorVk(const std::string &ipc_cmd_memory_segment = IpcMemory::DEFAULT_IPC_CMD_MEMORY_NAME.data(),
		                     const std::string &ipc_map_memory_segment = IpcMemory::DEFAULT_IPC_MAP_MEMORY_NAME.data());
		~IpcMemoryProcessorVk();

		void InitializeVulkan();
		void CleanupVulkan();

		char ProcessCmd(uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);
		void CleanupLocks();

	private:
		TextureShareVk _vk_data;
		std::map<IpcMemory::IMAGE_NAME_T, SharedImageVk> _image_data;

		char ProcessImageInitCmd(const IpcCmdImageInit &ipc_cmd);
		char ProcessRenameCmd(const IpcCmdRename &ipc_cmd);
		char ProcessHandleRequestCmd(const IpcCmdRequestImageHandles &ipc_cmd);
};

#endif // IPC_MEMORY_PROCESSOR_VK_H
