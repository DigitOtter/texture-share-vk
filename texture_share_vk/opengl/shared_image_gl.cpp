#include "texture_share_vk/opengl/shared_image_gl.h"

#include <utility>


bool SharedImageGl::InitializeGLExternal()
{
	// TODO: Load extensions
	return true;
}

void SharedImageGl::InitializeWithExternal(ExternalHandle::ShareHandles &&share_handles,
                                           GLsizei width, GLsizei height, GLenum internal_format,
                                           GLuint64 allocation_size)
{
	this->_share_handles = std::move(share_handles);

	//glDisable(GL_DEPTH_TEST);

	// Create the texture for the FBO color attachment.
	// This only reserves the ID, it doesn't allocate memory
	glGenTextures(1, &this->_color);
	glBindTexture(GL_TEXTURE_2D, this->_color);

	// Create the GL identifiers

	// semaphores
	glGenSemaphoresEXT(1, &this->_semaphore_read);
	glGenSemaphoresEXT(1, &this->_semaphore_write);
	// memory
	glCreateMemoryObjectsEXT(1, &this->_mem);

	// Platform specific import.
	ExternalHandleGl::ImportSemaphoreExt(this->_semaphore_read, ExternalHandleGl::GL_HANDLE_TYPE, this->_share_handles.ext_read);
	ExternalHandleGl::ImportSemaphoreExt(this->_semaphore_write, ExternalHandleGl::GL_HANDLE_TYPE, this->_share_handles.ext_write);
	ExternalHandleGl::ImportMemoryExt(this->_mem, allocation_size, ExternalHandleGl::GL_HANDLE_TYPE, this->_share_handles.memory);

	// Use the imported memory as backing for the OpenGL texture.  The internalFormat, dimensions
	// and mip count should match the ones used by Vulkan to create the image and determine it's memory
	// allocation.
	glTextureStorageMem2DEXT(this->_color, 1, internal_format, width,
	                         height, this->_mem, 0);
	glBindTexture(GL_TEXTURE_2D, 0);
}

void SharedImageGl::ReadImage(GLuint dstName, GLenum dstTarget, GLint dstLevel, GLint dstX, GLint dstY, GLint dstZ)
{
	//glCopyImageSubData(this->_color, GLenum srcTarget​, GLint srcLevel​, GLint srcX​, GLint srcY​, GLint srcZ​, GLuint dstName​, GLenum dstTarget​, GLint dstLevel​, GLint dstX​, GLint dstY​, GLint dstZ​, GLsizei srcWidth​, GLsizei srcHeight​, GLsizei srcDepth​);
}
