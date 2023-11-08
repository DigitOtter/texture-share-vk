#pragma once

#include "platform/external_handle_gl.h"

#include <GL/gl.h>
#include <GL/glext.h>

struct SharedImageData
{
	uint32_t Id             = 0;
	GLsizei Width           = 0;
	GLsizei Height          = 0;
	GLenum Format           = GL_FALSE;
	GLuint64 AllocationSize = 0;
};

struct ImageExtent
{
	GLsizei top_left[2];
	GLsizei bottom_right[2];
};

/*!
 * \brief Imports image texture from vulkan. Code adapted from
 * https://github.com/KhronosGroup/Vulkan-Samples, open_gl_interop
 */
class GlSharedImage
{
	static constexpr GLuint SHARED_IMAGE_TEX_TARGET = GL_TEXTURE_2D;

	public:
	using ImageExtent = ::ImageExtent;
	GlSharedImage();
	~GlSharedImage();

	GlSharedImage(const GlSharedImage &)            = delete;
	GlSharedImage &operator=(const GlSharedImage &) = delete;

	GlSharedImage(GlSharedImage &&)            = default;
	GlSharedImage &operator=(GlSharedImage &&) = delete;

	static bool InitializeGLExternal();

	GLenum Initialize(GLsizei width, GLsizei height, uint64_t handle_id, GLuint64 allocation_size,
	                  GLenum format = GL_RGBA, GLenum internal_format = GL_RGBA8);

	GLenum InitializeWithExternal(ExternalHandle::ShareHandles &&share_handles, GLsizei width, GLsizei height,
	                              uint64_t handle_id, GLuint64 allocation_size, GLenum format = GL_RGBA,
	                              GLenum internal_format = GL_RGBA8);

	void Cleanup();

	void RecvBlitImage(GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions, bool invert,
	                   GLuint prev_fbo);
	void SendBlitImage(GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions, bool invert,
	                   GLuint prev_fbo);

	/*!
	 * \brief Clears the image with the given color
	 * \param clear_color Clear color.
	 * Should be in the format described by ImageFormat(), usually RGBA
	 */
	void ClearImage(const unsigned char *clear_color);

	/*!
	 * \brief Clears the image with the given color
	 * \param clear_color Clear color.
	 * Should be in the format described by ImageFormat(), usually RGBA
	 * \param format clear_color format
	 * \param type clear_color individual value type
	 */
	void ClearImage(const void *clear_color, GLenum format, GLenum type = GL_UNSIGNED_BYTE);

	constexpr GLenum ImageFormat() const
	{
		return this->_image_data.Format;
	}

	constexpr GLuint TextureId() const
	{
		return this->_image_texture;
	}

	constexpr GLsizei Width() const
	{
		return this->_image_data.Width;
	}

	constexpr GLsizei Height() const
	{
		return this->_image_data.Height;
	}

	constexpr uint64_t HandleId() const
	{
		return this->_image_data.Id;
	}

	constexpr SharedImageData &ImageData()
	{
		return this->_image_data;
	}

	constexpr const SharedImageData &ImageData() const
	{
		return this->_image_data;
	}

	private:
	// FBO to render image from/to (this is the recommended method of copying images)
	GLuint _fbo = 0;

	// Semaphores
	// GLuint _semaphore_read  = 0;
	// GLuint _semaphore_write = 0;

	// Memory Object
	GLuint _mem = 0;

	// Texture
	GLuint _image_texture = 0;

	SharedImageData _image_data;

	/*!
	 * \brief Blit Image. Copy and scale image from src to dst.
	 * Code adapted from https://github.com/Off-World-Live/Spout2/blob/master/SPOUTSDK/SpoutGL/SpoutGL.cpp,
	 * spoutGL::GetSharedTextureData
	 * \param src_texture_id
	 * \param src_texture_target Source Texture mapping. If unsure, should most likely be GL_TEXTURE_2D
	 * \param src_dimensions Source image dimensions
	 * \param dst_texture_id
	 * \param dst_texture_target Dest Texture mapping. If unsure, should most likely be GL_TEXTURE_2D
	 * \param dst_dimensions Dest image dimensions
	 * \param invert Flip the image upside down
	 * \param prev_fbo Previous FBO
	 */
	void BlitImage(GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions,
	               GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions,
	               bool invert = false, GLuint prev_fbo = 0);
};
