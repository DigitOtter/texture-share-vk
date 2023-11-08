#pragma once

#include "gl_shared_image/gl_shared_image.h"
#include "platform/linux/external_handle.h"
// #include "gl_shared_image/platform/linux/external_handle_gl.h"
#include <GL/gl.h>
#include <GL/glext.h>
#include <cstdint>
#include <memory>

using ::GLenum;
using ::GLsizei;
using ::GLuint;
using ::GLuint64;

enum GlFormat
{
	RGBA  = GL_RGBA,
	BGRA  = GL_BGRA,
	RGB   = GL_RGB,
	BGR   = GL_BGR,
	FALSE = GL_FALSE,
};

class ShareHandlesWrapper : public ExternalHandle::ShareHandles

{
	public:
	ShareHandlesWrapper()  = default;
	~ShareHandlesWrapper() = default;

	ShareHandlesWrapper(ExternalHandle::ShareHandles &&handles)
		: ExternalHandle::ShareHandles(std::move(handles))
	{}

	constexpr ExternalHandle::TYPE get_memory_handle() const
	{
		return this->memory;
	}

	ExternalHandle::TYPE release_memory_handle()
	{
		const auto res = std::move(this->memory);
		this->memory   = ExternalHandle::INVALID_VALUE;

		return res;
	}
};

using ImageExtent = GlSharedImage::ImageExtent;

class GlSharedImageWrapper : public GlSharedImage
{
	public:
	GlSharedImageWrapper()  = default;
	~GlSharedImageWrapper() = default;

	static bool initialize_gl_external()
	{
		return GlSharedImage::InitializeGLExternal();
	}

	GLenum initialize(GLsizei width, GLsizei height, uint64_t handle_id, GLuint64 allocation_size, GlFormat format,
	                  GLenum internal_format)
	{
		return this->Initialize(width, height, handle_id, allocation_size, format, internal_format);
	}

	GLenum import_from_handle(std::unique_ptr<ShareHandlesWrapper> share_handles, const SharedImageData &image_data)
	{
		return this->InitializeWithExternal(std::move(*share_handles), image_data.Width, image_data.Height,
		                                    image_data.Id, image_data.AllocationSize);
	}

	void cleanup()
	{
		return this->Cleanup();
	}

	void recv_image_blit_with_extents(GLuint src_texture_id, GLuint src_texture_target,
	                                  const ImageExtent &src_dimensions, bool invert, GLuint prev_fbo)
	{
		return this->RecvBlitImage(src_texture_id, src_texture_target, src_dimensions, invert, prev_fbo);
	}

	void recv_image_blit(GLuint src_texture_id, GLuint src_texture_target, bool invert, GLuint prev_fbo)
	{
		ImageExtent src_dimensions = {
			{0,					  0					  },
			{(GLsizei)this->Width(), (GLsizei)this->Height()}
        };
		return this->RecvBlitImage(src_texture_id, src_texture_target, src_dimensions, invert, prev_fbo);
	}

	void send_image_blit_with_extents(GLuint dst_texture_id, GLuint dst_texture_target,
	                                  const ImageExtent &dst_dimensions, bool invert, GLuint prev_fbo)
	{
		return this->SendBlitImage(dst_texture_id, dst_texture_target, dst_dimensions, invert, prev_fbo);
	}

	void send_image_blit(GLuint dst_texture_id, GLuint dst_texture_target, bool invert, GLuint prev_fbo)
	{
		ImageExtent dst_dimensions = {
			{0,					  0					  },
			{(GLsizei)this->Width(), (GLsizei)this->Height()}
        };
		return this->SendBlitImage(dst_texture_id, dst_texture_target, dst_dimensions, invert, prev_fbo);
	}

	constexpr GLuint get_texture_id() const
	{
		return this->TextureId();
	}

	constexpr const SharedImageData &get_image_data() const
	{
		return this->ImageData();
	}

	constexpr SharedImageData &get_image_data_mut()
	{
		return this->ImageData();
	}
};

std::unique_ptr<GlSharedImageWrapper> gl_shared_image_new();

std::unique_ptr<ShareHandlesWrapper> gl_share_handles_new();
std::unique_ptr<ShareHandlesWrapper> gl_share_handles_from_fd(int memory_fd);

bool gl_external_initialize();

// using ImageExtent = GlSharedImage::ImageExtent;
