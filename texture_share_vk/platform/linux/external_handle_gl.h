#ifndef EXTERNAL_HANDLE_GL_H
#define EXTERNAL_HANDLE_GL_H

#include "texture_share_vk/platform/platform.h"

#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>
#include <GL/glext.h>


class ExternalHandleGl
{
	public:
		using TYPE = ExternalHandle::TYPE;
		static constexpr TYPE INVALID_VALUE = ExternalHandle::INVALID_VALUE;

		static constexpr GLenum GL_HANDLE_TYPE = GL_HANDLE_TYPE_OPAQUE_FD_EXT;

		static GLenum GetGlFormat(ExternalHandle::ImageFormat format);
		static GLenum GetGlInternalFormat(ExternalHandle::ImageFormat format);
		static ExternalHandle::ImageFormat GetImageFormat(GLenum gl_format);

		static GLuint GetGlFormatSize(GLenum gl_format);

		static void ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type, ExternalHandle::TYPE ext_semaphore_handle);
		static void ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type, ExternalHandle::TYPE ext_semaphore_handle);

	private:

};

#endif //EXTERNAL_HANDLE_GL_H
