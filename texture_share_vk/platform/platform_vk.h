#ifndef PLATFORM_VK_H
#define PLATFORM_VK_H

#if defined(WIN32)
#include "texture_share_vk/platform/win32/external_handle_vk.h"
#else
#include "texture_share_vk/platform/linux/external_handle_vk.h"
#endif

#endif //PLATFORM_VK_H
