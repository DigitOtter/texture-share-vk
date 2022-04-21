#include "texture_share_vk/daemon/texture_share_daemon.h"

#include "texture_share_vk/platform/config.h"

#include <chrono>
#include <iostream>
#include <thread>


TextureShareDaemon::TextureShareDaemon(const std::string &ipc_cmd_memory_segment,
                                       const std::string &ipc_map_memory_segment)
    : _lock_file(TSV_DAEMON_LOCK_FILE),
      _vk_memory(ipc_cmd_memory_segment,
                 ipc_map_memory_segment)
{}

void TextureShareDaemon::Initialize()
{
	this->_vk_memory.InitializeVulkan();
}

int TextureShareDaemon::Loop()
{
	volatile bool run = true;
	return this->Loop(run);
}

int TextureShareDaemon::Loop(volatile bool &run)
{
	while(run)
	{
		const auto ret_val = this->_vk_memory.ProcessCmd(1 * 1000 * 1000);

		if(ret_val != -3)
			std::cerr << "Processes command with result: " << (int)ret_val << std::endl;

		this->_vk_memory.CleanupLocks();
	}

	return 0;
}

int TextureShareDaemon::Cleanup()
{
	this->_vk_memory.CleanupVulkan();

	return 0;
}
