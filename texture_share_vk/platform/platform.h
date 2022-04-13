#ifndef PLATFORM_H
#define PLATFORM_H

#if defined(WIN32)
#include "texture_share_vk/platform/win32/external_handle.h"
#else
#include "texture_share_vk/platform/linux/external_handle.h"
#endif


#endif //PLATFORM_H
