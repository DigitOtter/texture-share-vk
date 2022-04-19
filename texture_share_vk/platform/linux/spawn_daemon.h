#ifndef TSV_SPAWN_DAEMON_H
#define TSV_SPAWN_DAEMON_H

#include <string>


class SpawnDaemon
{
	public:
		static void Daemonize(const std::string &ipc_cmd_memory_segment,
		                      const std::string &ipc_map_memory_segment);
};

#endif //TSV_SPAWN_DAEMON_H
