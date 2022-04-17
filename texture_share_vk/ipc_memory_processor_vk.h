#ifndef IPC_MEMORY_PROCESSOR_VK_H
#define IPC_MEMORY_PROCESSOR_VK_H

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/texture_share_vk.h"

#include <map>


class IpcMemoryProcessorVk
        : public IpcMemory
{
	public:
		IpcMemoryProcessorVk() = default;
		~IpcMemoryProcessorVk();

		void InitializeVulkan();
		void CleanupVulkan();

		bool ProcessCmd(uint64_t micro_sec_wait_time = DEFAULT_CMD_WAIT_TIME);

	private:
		TextureShareVk _vk_data;
		std::map<IpcMemory::IMAGE_NAME_T, SharedImageVk> _image_data;

		char ProcessNameChangeCmd(IpcCmdData &ipc_cmd);
		char ProcessHandleRequestCmd(IpcCmdData &ipc_cmd);
};

#endif // IPC_MEMORY_PROCESSOR_VK_H
