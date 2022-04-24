#include "external_handle_gl.h"


GLenum ExternalHandleGl::GetGlFormat(ExternalHandle::ImageFormat format)
{
	switch(format)
	{
		case ExternalHandle::ImageFormat::R8G8B8A8:
			return GL_RGBA;
		default:
			return 0;
	}
}

GLenum ExternalHandleGl::GetGlInternalFormat(ExternalHandle::ImageFormat format)
{
	switch(format)
	{
		case ExternalHandle::ImageFormat::R8G8B8A8:
			return GL_RGBA8;
		default:
			return 0;
	}
}

ExternalHandle::ImageFormat ExternalHandleGl::GetImageFormat(GLenum gl_format)
{
	switch(gl_format)
	{
		case GL_RGBA:
			return ExternalHandle::ImageFormat::R8G8B8A8;
		default:
			return ExternalHandle::ImageFormat::IMAGE_FORMAT_MAX_ENUM;
	}
}

GLuint ExternalHandleGl::GetGlFormatSize(GLenum gl_format)
{
	switch(gl_format)
	{
		case GL_RGBA:
			return 4;
		default:
			return 0;
	}
}

void ExternalHandleGl::ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	return glImportSemaphoreFdEXT(gl_semaphore, gl_handle_type, ext_semaphore_handle);
}

void ExternalHandleGl::ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	return glImportMemoryFdEXT(memory, size, handle_type, ext_semaphore_handle);
}
