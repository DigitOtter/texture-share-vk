#include "gl_shared_image.h"

#include <GL/gl.h>

#define GL_CHECK(x, e, l)                     \
	{                                         \
		x;                                    \
		if((e = glGetError()) != GL_NO_ERROR) \
			goto l;                           \
	}

GlSharedImage::GlSharedImage()
{
	// glGenFramebuffers(1, &this->_fbo);
}

GlSharedImage::~GlSharedImage()
{
	this->Cleanup();
}

bool GlSharedImage::InitializeGLExternal()
{
	return ExternalHandleGl::LoadGlEXT();
}

// void SharedImageHandleGl::InitializeWithExternal(ExternalHandle::SharedImageInfo &&external_handles)
// {
// 	const GLenum gl_format          = ExternalHandleGl::GetGlFormat(external_handles.format);
// 	const GLenum gl_internal_format = ExternalHandleGl::GetGlInternalFormat(external_handles.format);

// 	return this->InitializeWithExternal(std::move(external_handles.handles), external_handles.width,
// 	                                    external_handles.height, external_handles.handle_id,
// 	                                    external_handles.allocation_size, gl_format, gl_internal_format);
// }

GLenum GlSharedImage::Initialize(GLsizei width, GLsizei height, uint64_t handle_id, GLuint64 allocation_size,
                                 GLenum format, GLenum internal_format)
{
	this->Cleanup();

	GLenum error = GL_NO_ERROR;

	GL_CHECK(glGenTextures(1, &this->_image_texture), error, finish_label);
	GL_CHECK(glBindTexture(SHARED_IMAGE_TEX_TARGET, this->_image_texture), error, finish_label);

	GL_CHECK(glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST), error, finish_label);
	GL_CHECK(glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST), error, finish_label);
	GL_CHECK(glTexImage2D(GL_TEXTURE_2D, 0, internal_format, width, height, 0, format, GL_UNSIGNED_BYTE, nullptr),
	         error, finish_label);

finish_label:
	this->_image_data.Width          = width;
	this->_image_data.Height         = height;
	this->_image_data.Format         = format;
	this->_image_data.AllocationSize = allocation_size;
	this->_image_data.Id             = handle_id;

	glBindTexture(SHARED_IMAGE_TEX_TARGET, 0);

	return error;
}

GLenum GlSharedImage::InitializeWithExternal(ExternalHandle::ShareHandles &&share_handles, GLsizei width,
                                             GLsizei height, uint64_t handle_id, GLuint64 allocation_size,
                                             GLenum format, GLenum internal_format)
{
	this->Cleanup();

	GLenum error = GL_NO_ERROR;

	// Create the texture for the FBO color attachment.
	// This only reserves the ID, it doesn't allocate memory
	GL_CHECK(glGenTextures(1, &this->_image_texture), error, finish_label);
	GL_CHECK(glBindTexture(SHARED_IMAGE_TEX_TARGET, this->_image_texture), error, finish_label);

	// Create the GL identifiers

	// semaphores
	// GL_CHECK(ExternalHandleGl::GenSemaphoresEXT(1, &this->_semaphore_read), error, finish_label);
	// GL_CHECK(ExternalHandleGl::GenSemaphoresEXT(1, &this->_semaphore_write), error, finish_label);

	// memory
	GL_CHECK(ExternalHandleGl::CreateMemoryObjectsEXT(1, &this->_mem), error, finish_label);

	// Platform specific import.
	// ExternalHandleGl::ImportSemaphoreExt(this->_semaphore_read, ExternalHandleGl::GL_HANDLE_TYPE,
	//                                      share_handles.ext_read);
	// share_handles.ext_read  = -1;
	// ExternalHandleGl::ImportSemaphoreExt(this->_semaphore_write, ExternalHandleGl::GL_HANDLE_TYPE,
	//                                      share_handles.ext_write);
	// share_handles.ext_write = -1;
	GL_CHECK(ExternalHandleGl::ImportMemoryExt(this->_mem, allocation_size, ExternalHandleGl::GL_HANDLE_TYPE,
	                                           share_handles.memory),
	         error, finish_label);
	share_handles.memory = -1;

	// Use the imported memory as backing for the OpenGL texture.  The internalFormat, dimensions
	// and mip count should match the ones used by Vulkan to create the image and determine it's memory
	// allocation.
	GL_CHECK(ExternalHandleGl::TextureStorageMem2DEXT(this->_image_texture, 1, internal_format, width, height,
	                                                  this->_mem, 0),
	         error, finish_label);

finish_label:
	this->_image_data.Width          = width;
	this->_image_data.Height         = height;
	this->_image_data.Format         = format;
	this->_image_data.Id             = handle_id;
	this->_image_data.AllocationSize = allocation_size;

	glBindTexture(SHARED_IMAGE_TEX_TARGET, 0);

	return error;
}

void GlSharedImage::Cleanup()
{
	if(this->_image_texture > 0)
	{
		glDeleteTextures(1, &this->_image_texture);
		this->_image_texture = 0;
	}

	if(this->_mem > 0)
	{
		ExternalHandleGl::DeleteMemoryObjectsEXT(1, &this->_mem);
		this->_mem = 0;
	}

	// if(this->_semaphore_write)
	// {
	// 	ExternalHandleGl::DeleteSemaphoresEXT(1, &this->_semaphore_write);
	// 	this->_semaphore_write = 0;
	// }

	// if(this->_semaphore_read)
	// {
	// 	ExternalHandleGl::DeleteSemaphoresEXT(1, &this->_semaphore_read);
	// 	this->_semaphore_read = 0;
	// }

	if(this->_fbo > 0)
	{
		glDeleteFramebuffers(1, &this->_fbo);
		this->_fbo = 0;
	}
}

void GlSharedImage::RecvBlitImage(GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions,
                                  bool invert, GLuint prev_fbo)
{
	return this->BlitImage(src_texture_id, src_texture_target, src_dimensions, this->_image_texture,
	                       SHARED_IMAGE_TEX_TARGET,
	                       ImageExtent{
							   {0,                       0                       },
							   {this->_image_data.Width, this->_image_data.Height}
    },
	                       invert, prev_fbo);
}

void GlSharedImage::SendBlitImage(GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions,
                                  bool invert, GLuint prev_fbo)
{
	return this->BlitImage(this->_image_texture, SHARED_IMAGE_TEX_TARGET,
	                       ImageExtent{
							   {0,                       0                       },
							   {this->_image_data.Width, this->_image_data.Height}
    },
	                       dst_texture_id, dst_texture_target, dst_dimensions, invert, prev_fbo);
}

void GlSharedImage::ClearImage(const unsigned char *clear_color)
{
	return this->ClearImage(clear_color, this->_image_data.Format, GL_UNSIGNED_BYTE);
}

void GlSharedImage::ClearImage(const void *clear_color, GLenum format, GLenum type)
{
	glClearTexImage(this->_image_texture, 0, format, type, clear_color);
}

void GlSharedImage::BlitImage(GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions,
                              GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions,
                              bool invert, GLuint prev_fbo)
{
	if(this->_fbo == 0)
	{
		glGenFramebuffers(1, &this->_fbo);
	}

	// bind the FBO (for both, READ_FRAMEBUFFER_EXT and DRAW_FRAMEBUFFER_EXT)
	glBindFramebuffer(GL_FRAMEBUFFER, this->_fbo);

	// Attach the Input texture (the shared texture) to the color buffer in our frame buffer - note texturetarget
	glFramebufferTexture2D(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, src_texture_target, src_texture_id, 0);
	glReadBuffer(GL_COLOR_ATTACHMENT0_EXT);

	// Attach target texture (the one we write into and return) to second attachment point
	glFramebufferTexture2D(GL_DRAW_FRAMEBUFFER, GL_COLOR_ATTACHMENT1, dst_texture_target, dst_texture_id, 0);

	glDrawBuffer(GL_COLOR_ATTACHMENT1);

	// Check read/draw fbo for completeness
	GLuint status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
	if(status == GL_FRAMEBUFFER_COMPLETE_EXT)
	{
		//		if (m_bBLITavailable)
		//		{
		// Flip if the user wants that
		if(!invert)
		{
			// Do not flip during blit
			glBlitFramebuffer(src_dimensions.top_left[0], src_dimensions.top_left[1],         // srcX0, srcY0,
			                  src_dimensions.bottom_right[0], src_dimensions.bottom_right[1], // srcX1, srcY1
			                  dst_dimensions.top_left[0], dst_dimensions.top_left[1],         // dstX0, dstY0,
			                  dst_dimensions.bottom_right[0], dst_dimensions.bottom_right[1], // dstX1, dstY1
			                  GL_COLOR_BUFFER_BIT, GL_LINEAR);
		}
		else
		{
			// copy one texture buffer to the other while flipping upside down
			glBlitFramebuffer(src_dimensions.top_left[0], src_dimensions.top_left[1],         // srcX0, srcY0,
			                  src_dimensions.bottom_right[0], src_dimensions.bottom_right[1], // srcX1, srcY1
			                  dst_dimensions.top_left[0], dst_dimensions.bottom_right[1],     // dstX0, dstY0,
			                  dst_dimensions.bottom_right[0], dst_dimensions.top_left[1],     // dstX1, dstY1
			                  GL_COLOR_BUFFER_BIT, GL_LINEAR);
		}
		//	    }
		//	    else {
		//		    // No fbo blit extension available
		//		    // Copy from the fbo (shared texture attached) to the dest texture
		//		    glBindTexture(TextureTarget, TextureID);
		//			glCopyTexSubImage2D(TextureTarget, 0, 0, 0, 0, 0, width, height);
		//			glBindTexture(TextureTarget, 0);
		//	    }
		//	    bRet = true;
	}
	else
	{
		//		PrintFBOstatus(status);
		//		bRet = false;
	}

	//	std::array<uint8_t, 1920 * 1080 * 4> data;
	//	data.fill(0);
	//	glGetTextureImage(src_texture_id, 0, GL_BGRA, GL_UNSIGNED_BYTE, data.size(), data.data());
	//	code = glGetError();

	//	std::array<uint8_t, 1920 * 1080 * 4> rec_data;
	//	rec_data.fill(0);
	//	glGetTextureImage(src_texture_id, 0, GL_BGRA, GL_UNSIGNED_BYTE, rec_data.size(), rec_data.data());
	//	code = glGetError();

	//	int unequal = 0;
	//	for(size_t i = 0; i < data.size(); ++i)
	//	{
	//			if(data.at(i) != rec_data.at(i))
	//				++unequal;
	//	}

	// restore the previous fbo - default is 0
	glDrawBuffer(GL_COLOR_ATTACHMENT0_EXT); // 04.01.16
	glBindFramebuffer(GL_FRAMEBUFFER, prev_fbo);
}
