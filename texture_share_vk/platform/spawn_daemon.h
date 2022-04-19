#ifndef PLATFORM_TSV_SPAWN_DAEMON_H
#define PLATFORM_TSV_SPAWN_DAEMON_H

#ifdef WIN32
#include "texture_share_vk/platform/win32/spawn_daemon.h"
#else
#include "texture_share_vk/platform/linux/spawn_daemon.h"
#endif

#endif // PLATFORM_TSV_SPAWN_DAEMON_H
