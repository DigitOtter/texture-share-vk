#ifndef TSV_DAEMON_COMM_H
#define TSV_DAEMON_COMM_H

#include <string>


class DaemonComm
{
	public:
		static void Daemonize(const std::string &ipc_cmd_memory_segment,
		                      const std::string &ipc_map_memory_segment);
};

#endif //TSV_DAEMON_COMM_H
