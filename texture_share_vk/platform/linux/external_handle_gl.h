#ifndef EXTERNAL_HANDLE_GL_H
#define EXTERNAL_HANDLE_GL_H

#include "texture_share_vk/platform/platform.h"

#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>
#include <GL/glext.h>


class ExternalHandleGl
{
	public:
		using TYPE = ExternalHandleVk::TYPE;
		static constexpr TYPE INVALID_VALUE = ExternalHandleVk::INVALID_VALUE;

		static constexpr GLenum GL_HANDLE_TYPE = GL_HANDLE_TYPE_OPAQUE_FD_EXT;

		static void ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type, ExternalHandleVk::TYPE ext_semaphore_handle);
		static void ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type, ExternalHandleVk::TYPE ext_semaphore_handle);

	private:

};

#endif //EXTERNAL_HANDLE_GL_H
