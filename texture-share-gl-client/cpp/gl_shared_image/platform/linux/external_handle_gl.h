#ifndef EXTERNAL_HANDLE_GL_H
#define EXTERNAL_HANDLE_GL_H

#include "external_handle.h"

#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>
#include <GL/glext.h>

class ExternalHandleGl
{
	public:
	using TYPE                          = ExternalHandle::TYPE;
	static constexpr TYPE INVALID_VALUE = ExternalHandle::INVALID_VALUE;

	static constexpr GLenum GL_HANDLE_TYPE = GL_HANDLE_TYPE_OPAQUE_FD_EXT;

	static bool LoadGlEXT();

	static GLuint GetGlFormatSize(GLenum gl_format);

	static void ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type,
	                               ExternalHandle::TYPE ext_semaphore_handle);
	static void ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type,
	                            ExternalHandle::TYPE ext_semaphore_handle);
	static void DeleteSemaphoresEXT(GLsizei n, const GLuint *semaphores);

	static void GenSemaphoresEXT(GLsizei n, const GLuint *semaphores);
	static void CreateMemoryObjectsEXT(GLsizei n, GLuint *memoryObjects);
	static void DeleteMemoryObjectsEXT(GLsizei n, const GLuint *memoryObjects);

	static void TextureStorageMem2DEXT(GLuint texture, GLsizei levels, GLenum internalFormat, GLsizei width,
	                                   GLsizei height, GLuint memory, GLuint64 offset);

	private:
	using gen_semaphores_fcn_t = void(GLsizei n, const GLuint *semaphores);
	static gen_semaphores_fcn_t *gen_semaphores_fcn;

	using import_semaphore_fcn_fd_t = void(GLuint gl_semaphore, GLenum gl_handle_type,
	                                       ExternalHandle::TYPE ext_semaphore_handle);
	static import_semaphore_fcn_fd_t *import_semaphore_fd_fcn;

	using delete_semaphores_fcn_t = void(GLsizei n, const GLuint *semaphores);
	static delete_semaphores_fcn_t *delete_semaphores_fcn;

	using create_memory_objects_fcn_t = void(GLsizei n, GLuint *memoryObjects);
	static create_memory_objects_fcn_t *create_memory_objects_fcn;

	using import_memory_fd_fcn_t = void(GLuint memory, GLuint64 size, GLenum handle_type,
	                                    ExternalHandle::TYPE ext_semaphore_handle);
	static import_memory_fd_fcn_t *import_memory_fd_fcn;

	using delete_memory_objects_fcn_t = void(GLsizei n, const GLuint *memoryObjects);
	static delete_memory_objects_fcn_t *delete_memory_objects_fcn;

	using texture_storage_mem_2d_fcn_t = void(GLuint texture, GLsizei levels, GLenum internalFormat, GLsizei width,
	                                          GLsizei height, GLuint memory, GLuint64 offset);
	static texture_storage_mem_2d_fcn_t *texture_storage_mem_2d_fcn;
};

#endif // EXTERNAL_HANDLE_GL_H
