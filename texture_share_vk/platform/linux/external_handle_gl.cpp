#include "external_handle_gl.h"

void ExternalHandleGl::ImportSemaphoreExt(GLuint gl_semaphore, GLenum gl_handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	return glImportSemaphoreFdEXT(gl_semaphore, gl_handle_type, ext_semaphore_handle);
}

void ExternalHandleGl::ImportMemoryExt(GLuint memory, GLuint64 size, GLenum handle_type, ExternalHandle::TYPE ext_semaphore_handle)
{
	return glImportMemoryFdEXT(memory, size, handle_type, ext_semaphore_handle);
}
