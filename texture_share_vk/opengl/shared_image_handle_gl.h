#ifndef SHARED_IMAGE_HANDLE_GL_H
#define SHARED_IMAGE_HANDLE_GL_H

#include "texture_share_vk/platform/platform_gl.h"

#include <sys/types.h>


/*!
 * \brief Imports image texture from vulkan. Code adapted from
 * https://github.com/KhronosGroup/Vulkan-Samples, open_gl_interop
 */
class SharedImageHandleGl
{
		static constexpr GLuint SHARED_IMAGE_TEX_TARGET = GL_TEXTURE_2D;

	public:
		struct ImageExtent
		{
			GLsizei top_left[2];
			GLsizei bottom_right[2];
		};

		SharedImageHandleGl();
		~SharedImageHandleGl();

		SharedImageHandleGl(const SharedImageHandleGl&) = delete;
		SharedImageHandleGl &operator=(const SharedImageHandleGl&) = delete;

		SharedImageHandleGl(SharedImageHandleGl &&)            = default;
		SharedImageHandleGl &operator=(SharedImageHandleGl &&) = delete;

		static bool InitializeGLExternal();

		void InitializeWithExternal(ExternalHandle::SharedImageInfo &&external_handles);
		void InitializeWithExternal(ExternalHandle::ShareHandles &&share_handles, GLsizei width, GLsizei height,
	                                uint64_t handle_id, GLuint64 allocation_size, GLenum format = GL_RGBA,
	                                GLenum internal_format = GL_RGBA8);

		void Cleanup();

		void SendBlitImage(GLuint src_texture_id, GLuint src_texture_target, const ImageExtent &src_dimensions, bool invert, GLuint prev_fbo);
		void RecvBlitImage(GLuint dst_texture_id, GLuint dst_texture_target, const ImageExtent &dst_dimensions, bool invert, GLuint prev_fbo);

		/*!
		 * \brief Clears the image with the given color
		 * \param clear_color Clear color.
		 * Should be in the format described by ImageFormat(), usually RGBA
		 */
		void ClearImage(const u_char *clear_color);

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
			return this->_image_format;
		}

		constexpr GLuint TextureId() const
		{
			return this->_image_texture;
		}

		constexpr GLsizei Width() const
		{
			return this->_width;
		}

		constexpr GLsizei Height() const
		{
			return this->_height;
		}

		constexpr uint64_t HandleId() const
		{
			return this->_handle_id;
		}

		private:
		// FBO to render image from/to (this is the recommended method of copying images)
		GLuint _fbo = 0;

		// Semaphores
		GLuint _semaphore_read = 0;
		GLuint _semaphore_write = 0;

		// Memory Object
		GLuint _mem = 0;

		// Texture
		GLuint _image_texture = 0;

		GLsizei _width = 0, _height = 0;
		GLenum _image_format = GL_RGBA;
		uint64_t _handle_id  = 0;

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

#endif //SHARED_IMAGE_HANDLE_GL_H
