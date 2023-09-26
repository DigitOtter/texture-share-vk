#include "external_handle_gl.h"

#include <GL/glx.h>
#include <assert.h>

ExternalHandleGl::gen_semaphores_fcn_t *ExternalHandleGl::gen_semaphores_fcn                 = nullptr;
ExternalHandleGl::import_semaphore_fcn_fd_t *ExternalHandleGl::import_semaphore_fd_fcn       = nullptr;
ExternalHandleGl::delete_semaphores_fcn_t *ExternalHandleGl::delete_semaphores_fcn           = nullptr;
ExternalHandleGl::create_memory_objects_fcn_t *ExternalHandleGl::create_memory_objects_fcn   = nullptr;
ExternalHandleGl::import_memory_fd_fcn_t *ExternalHandleGl::import_memory_fd_fcn             = nullptr;
ExternalHandleGl::delete_memory_objects_fcn_t *ExternalHandleGl::delete_memory_objects_fcn   = nullptr;
ExternalHandleGl::texture_storage_mem_2d_fcn_t *ExternalHandleGl::texture_storage_mem_2d_fcn = nullptr;

bool ExternalHandleGl::LoadGlEXT()
{
	gen_semaphores_fcn        = (gen_semaphores_fcn_t *)glXGetProcAddress((GLubyte *)"glGenSemaphoresEXT");
	import_semaphore_fd_fcn   = (import_semaphore_fcn_fd_t *)glXGetProcAddress((GLubyte *)"glImportSemaphoreFdEXT");
	delete_semaphores_fcn     = (delete_semaphores_fcn_t *)glXGetProcAddress((GLubyte *)"glDeleteSemaphoresEXT");
	create_memory_objects_fcn = (create_memory_objects_fcn_t *)glXGetProcAddress((GLubyte *)"glCreateMemoryObjectsEXT");
	import_memory_fd_fcn      = (import_memory_fd_fcn_t *)glXGetProcAddress((GLubyte *)"glImportMemoryFdEXT");
	delete_memory_objects_fcn = (delete_memory_objects_fcn_t *)glXGetProcAddress((GLubyte *)"glDeleteMemoryObjectsEXT");
	texture_storage_mem_2d_fcn =
		(texture_storage_mem_2d_fcn_t *)glXGetProcAddress((GLubyte *)"glTextureStorageMem2DEXT");

	return (gen_semaphores_fcn && import_semaphore_fd_fcn && delete_semaphores_fcn && create_memory_objects_fcn &&
	        import_memory_fd_fcn && delete_memory_objects_fcn && texture_storage_mem_2d_fcn);
}

GLenum ExternalHandleGl::GetGlFormat(ExternalHandle::ImageFormat format)
{
	switch(format)
	{
		case ExternalHandle::ImageFormat::B8G8R8A8:
			return GL_BGRA;
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
		case ExternalHandle::ImageFormat::B8G8R8A8:
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
		case GL_BGRA:
			return ExternalHandle::ImageFormat::B8G8R8A8;
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
		case GL_BGRA:
		case GL_RGBA:
			return 4;
		default:
			return 0;
	}
}

void ExternalHandleGl::ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	assert(import_semaphore_fd_fcn);
	return import_semaphore_fd_fcn(gl_semaphore, gl_handle_type, ext_semaphore_handle);
}

void ExternalHandleGl::ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	assert(import_memory_fd_fcn);
	return import_memory_fd_fcn(memory, size, handle_type, ext_semaphore_handle);
}

void ExternalHandleGl::DeleteSemaphoresEXT(GLsizei n, const GLuint *semaphores)
{
	assert(delete_semaphores_fcn);
	return delete_semaphores_fcn(n, semaphores);
}

void ExternalHandleGl::GenSemaphoresEXT(GLsizei n, const GLuint *semaphores)
{
	assert(gen_semaphores_fcn);
	return gen_semaphores_fcn(n, semaphores);
}

void ExternalHandleGl::CreateMemoryObjectsEXT(GLsizei n, GLuint *memoryObjects)
{
	assert(create_memory_objects_fcn);
	return create_memory_objects_fcn(n, memoryObjects);
}

void ExternalHandleGl::DeleteMemoryObjectsEXT(GLsizei n, const GLuint *memoryObjects)
{
	assert(delete_memory_objects_fcn);
	return delete_memory_objects_fcn(n, memoryObjects);
}

void ExternalHandleGl::TextureStorageMem2DEXT(GLuint texture, GLsizei levels, GLenum internalFormat, GLsizei width,
                                              GLsizei height, GLuint memory, GLuint64 offset)
{
	assert(texture_storage_mem_2d_fcn);
	return texture_storage_mem_2d_fcn(texture, levels, internalFormat, width, height, memory, offset);
}
