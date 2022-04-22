#ifndef SHARED_IMAGE_HANDLE_GL_H
#define SHARED_IMAGE_HANDLE_GL_H

#include "texture_share_vk/platform/platform_gl.h"

class SharedImageHandleGl
{
	public:
		SharedImageHandleGl() = default;
		~SharedImageHandleGl() = default;

		static bool InitializeGLExternal();

		void InitializeWithExternal(ExternalHandle::ShareHandles &&share_handles,
		                            GLsizei width, GLsizei height, GLenum internal_format,
		                            GLuint64 allocation_size);

		void ReadImage(GLuint dstName, GLenum dstTarget, GLint dstLevel, GLint dstX, GLint dstY, GLint dstZ);

	private:
		ExternalHandle::ShareHandles _share_handles;

		// Semaphores
		GLuint _semaphore_read{0};
		GLuint _semaphore_write{0};

		// Memory Object
		GLuint _mem{0};

		// Texture
		GLuint _color{0};

		GLsizei _width{0}, _height{0};
		GLenum _image_format = GL_RGBA8;
};

#endif //SHARED_IMAGE_HANDLE_GL_H
