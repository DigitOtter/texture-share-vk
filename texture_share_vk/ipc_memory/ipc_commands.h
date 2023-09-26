#pragma once

#include "texture_share_vk/platform/daemon_comm.h"

#include <algorithm>
#include <array>
#include <unistd.h>

namespace ipc_commands
{
using IMAGE_NAME_T      = std::array<char, 1024>;
using SOCKET_FILENAME_T = std::array<char, 1024>;

static constexpr unsigned int IPC_QUEUE_MSG_PRIORITY_DEFAULT = 50;

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
	uint32_t imge_width                      = 0;
	uint32_t imge_height                     = 0;
	ExternalHandle::ImageFormat image_format = ExternalHandle::IMAGE_FORMAT_MAX_ENUM;
	bool overwrite_existing                  = false;
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

static constexpr size_t IPC_QUEUE_MSG_SIZE = std::max(
	{sizeof(IpcCmdImageInit), sizeof(IpcCmdRename), sizeof(IpcCmdRequestImageHandles), sizeof(IpcCmdRegisterProc)});

struct PipeHandleAck
{
	bool Ack = true;
};

struct ImageNameCompare
{
	bool operator()(const IMAGE_NAME_T &x, const IMAGE_NAME_T &y) const
	{
		constexpr size_t len = IMAGE_NAME_T().size();
		return strncmp(x.data(), y.data(), len) < 0;
	}
};
} // namespace ipc_commands
