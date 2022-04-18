#include "texture_share_vk/daemon/texture_share_daemon.h"

#include <chrono>
#include <iostream>
#include <thread>


TextureShareDaemon::TextureShareDaemon(const std::string &ipc_cmd_memory_segment,
                                       const std::string &ipc_map_memory_segment)
    : _vk_memory(ipc_cmd_memory_segment,
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
		if(this->_vk_memory.IsCmdRequestPresent())
		{
			const auto ret_val = this->_vk_memory.ProcessCmd();
			std::cerr << "Processes command with result: " << (unsigned char)ret_val << std::endl;
		}
		else
			std::this_thread::sleep_for(std::chrono::microseconds(DEFAULT_WAIT_TIME_MICRO_S));
	}

	return 0;
}

int TextureShareDaemon::Cleanup()
{
	this->_vk_memory.CleanupVulkan();

	return 0;
}
