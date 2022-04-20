#include "spawn_daemon.h"

#include "texture_share_vk/ipc_memory.h"
#include "texture_share_vk/platform/config.h"

#include <unistd.h>


void DaemonComm::Daemonize(const std::string &ipc_cmd_memory_segment, const std::string &ipc_map_memory_segment)
{
	if(IpcMemory::SharedMemoryExists(ipc_cmd_memory_segment))
		return;

	int c_pid = fork();
	if(c_pid == 0)
	{
		// Child process
		if(setsid() < 0)
			throw std::runtime_error("Failed to daemonize texture share daemon");

		const int ret = execlp(TSV_DAEMON_PATH,
		                       ipc_cmd_memory_segment.c_str(),
		                       ipc_map_memory_segment.c_str(),
		                       nullptr);
		exit(ret);
	}
	else if(c_pid < 0)
	{
		throw std::runtime_error("Failed to create texture share daemon");
	}
}
