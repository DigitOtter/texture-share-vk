#ifndef PLATFORM_GL_H
#define PLATFORM_GL_H

#if defined(WIN32)
#include "texture_share_vk/platform/win32/external_handle_gl.h"
#else
#include "texture_share_vk/platform/linux/external_handle_gl.h"
#endif

#endif //PLATFORM_GL_H
