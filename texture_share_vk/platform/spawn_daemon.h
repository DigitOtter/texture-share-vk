#ifndef PLATFORM_TSV_SPAWN_DAEMON_H
#define PLATFORM_TSV_SPAWN_DAEMON_H

#ifdef WIN32
#include "texture_share_vk/platform/win32/daemon_comm.h"
#else
#include "texture_share_vk/platform/linux/daemon_comm.h"
#endif

#endif // PLATFORM_TSV_SPAWN_DAEMON_H
