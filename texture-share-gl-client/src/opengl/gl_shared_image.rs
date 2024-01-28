use super::glad;
use std::{os::fd::OwnedFd, ptr};

use texture_share_ipc::platform::{img_data::ImgFormat, ShmemDataInternal};
#[cfg(target_os = "linux")]
type GlMemoryHandle = OwnedFd;

pub struct GlSharedImage {
	// FBO to render image from/to
	fbo: glad::GLuint,

	// Image memory
	mem: glad::GLuint,

	// Image texture
	texture: glad::GLuint,

	data: GlSharedImageData,
}

pub struct GlSharedImageData {
	pub id: u32,
	pub width: u32,
	pub height: u32,
	pub format: glad::GLenum,
	pub allocation_size: u64,
}

#[repr(C)]
pub struct GlImageExtent {
	pub top_left: [glad::GLint; 2],
	pub bottom_right: [glad::GLint; 2],
}

impl Drop for GlSharedImage {
	fn drop(&mut self) {
		if self.texture != 0 {
			unsafe { glad::glad_glDeleteTextures.unwrap()(1, &self.texture) };
			self.texture = 0;
		}

		if self.mem != 0 {
			unsafe { glad::glad_glDeleteMemoryObjectsEXT.unwrap()(1, &self.mem) };
			self.mem = 0;
		}

		if self.fbo != 0 {
			unsafe { glad::glad_glDeleteFramebuffers.unwrap()(1, &self.fbo) };
			self.fbo = 0;
		}
	}
}

impl GlSharedImageData {
	pub fn from_shmem_img_data(data: &ShmemDataInternal) -> GlSharedImageData {
		GlSharedImageData {
			id: data.handle_id,
			width: data.width,
			height: data.height,
			format: GlSharedImage::get_gl_format(data.format),
			allocation_size: data.allocation_size,
		}
	}
}

unsafe fn check_gl<F: FnOnce() -> T, T>(fcn: F) -> Result<T, glad::GLuint> {
	let ret = fcn();
	match glad::glad_glGetError.unwrap_unchecked()() {
		0 => Ok(ret),
		e => Err(e),
	}
}

impl GlSharedImage {
	pub fn init_gl() -> Result<i32, ()> {
		match unsafe { glad::gladLoadGL() } {
			0 => Err(()),
			v => Ok(v),
		}
	}

	pub fn get_gl_format(img_format: ImgFormat) -> glad::GLenum {
		match img_format {
			ImgFormat::B8G8R8 => glad::GL_BGR,
			ImgFormat::B8G8R8A8 => glad::GL_BGRA,
			ImgFormat::R8G8B8 => glad::GL_RGB,
			ImgFormat::R8G8B8A8 => glad::GL_RGBA,
			ImgFormat::Undefined => glad::GL_NONE,
		}
	}

	pub fn get_gl_internal_format(img_format: ImgFormat) -> glad::GLint {
		match img_format {
			ImgFormat::B8G8R8 => glad::GL_RGB8,
			ImgFormat::B8G8R8A8 => glad::GL_RGBA8,
			ImgFormat::R8G8B8 => glad::GL_RGB8,
			ImgFormat::R8G8B8A8 => glad::GL_RGBA8,
			ImgFormat::Undefined => glad::GL_NONE,
		}
		.try_into()
		.unwrap()
	}

	pub fn get_img_format(gl_format: glad::GLenum) -> ImgFormat {
		match gl_format {
			glad::GL_BGR => ImgFormat::B8G8R8,
			glad::GL_BGRA => ImgFormat::B8G8R8A8,
			glad::GL_RGB => ImgFormat::R8G8B8,
			glad::GL_RGBA => ImgFormat::R8G8B8A8,
			_ => ImgFormat::Undefined,
		}
	}

	pub fn new(
		width: glad::GLsizei,
		height: glad::GLsizei,
		allocation_size: glad::GLuint64,
		format: glad::GLenum,
		internal_format: glad::GLint,
		id: u32,
	) -> Result<GlSharedImage, u32> {
		let mut texture: glad::GLuint = 0;
		unsafe {
			check_gl(|| glad::glad_glGenTextures.unwrap()(1, &mut texture as *mut u32))?;
			check_gl(|| glad::glad_glBindTexture.unwrap()(glad::GL_TEXTURE_2D, texture))?;

			check_gl(|| {
				glad::glad_glTexParameteri.unwrap()(
					glad::GL_TEXTURE_2D,
					glad::GL_TEXTURE_MIN_FILTER,
					glad::GL_NEAREST as i32,
				)
			})?;
			check_gl(|| {
				glad::glad_glTexParameteri.unwrap()(
					glad::GL_TEXTURE_2D,
					glad::GL_TEXTURE_MAG_FILTER,
					glad::GL_NEAREST as i32,
				)
			})?;

			check_gl(|| {
				glad::glad_glTexImage2D.unwrap()(
					glad::GL_TEXTURE_2D,
					0,
					internal_format,
					width,
					height,
					0,
					format,
					glad::GL_UNSIGNED_BYTE,
					ptr::null(),
				)
			})?;
			check_gl(|| glad::glad_glBindTexture.unwrap()(glad::GL_TEXTURE_2D, 0))?;
		};

		let data = GlSharedImageData {
			id,
			width: width as u32,
			height: height as u32,
			format,
			allocation_size,
		};
		Ok(GlSharedImage {
			fbo: 0,
			mem: 0,
			texture,
			data,
		})
	}

	#[cfg(target_os = "linux")]
	pub fn import_handle(
		handle: GlMemoryHandle,
		width: glad::GLsizei,
		height: glad::GLsizei,
		allocation_size: glad::GLuint64,
		format: glad::GLenum,
		internal_format: glad::GLenum,
		id: u32,
	) -> Result<GlSharedImage, glad::GLuint> {
		use std::os::fd::IntoRawFd;

		let mut texture: glad::GLuint = 0;
		let mut memory: glad::GLuint = 0;

		unsafe {
			check_gl(|| glad::glad_glGenTextures.unwrap()(1, &mut texture as *mut _))?;
			check_gl(|| glad::glad_glBindTexture.unwrap()(glad::GL_TEXTURE_2D, texture))?;

			check_gl(|| glad::glad_glCreateMemoryObjectsEXT.unwrap()(1, &mut memory as *mut _))?;
			check_gl(|| {
				glad::glad_glImportMemoryFdEXT.unwrap()(
					memory,
					allocation_size,
					glad::GL_HANDLE_TYPE_OPAQUE_FD_EXT,
					handle.into_raw_fd(),
				)
			})?;
			check_gl(|| {
				glad::glad_glTextureStorageMem2DEXT.unwrap()(
					texture,
					1,
					internal_format,
					width,
					height,
					memory,
					0,
				)
			})?;

			check_gl(|| glad::glad_glBindTexture.unwrap()(glad::GL_TEXTURE_2D, 0))?;
		}

		let data = GlSharedImageData {
			id,
			width: width as u32,
			height: height as u32,
			format,
			allocation_size,
		};
		Ok(GlSharedImage {
			fbo: 0,
			mem: 0,
			texture,
			data,
		})
	}

	pub fn get_data(&self) -> &GlSharedImageData {
		&self.data
	}

	fn blit_image(
		src_texture: glad::GLuint,
		src_target: glad::GLuint,
		src_dimensions: &GlImageExtent,
		dst_texture: glad::GLuint,
		dst_target: glad::GLuint,
		dst_dimensions: &GlImageExtent,
		invert: bool,
		blit_fbo: glad::GLuint,
		prev_fbo: glad::GLuint,
	) -> Result<glad::GLuint, glad::GLuint> {
		let blit_fbo = match blit_fbo {
			0 => unsafe {
				let mut fbo: glad::GLuint = 0;
				check_gl(|| glad::glad_glGenFramebuffers.unwrap()(1, &mut fbo))?;
				fbo
			},
			id => id,
		};

		unsafe {
			// bind the FBO (for both, READ_FRAMEBUFFER_EXT and DRAW_FRAMEBUFFER_EXT)
			check_gl(|| {
				glad::glad_glBindFramebuffer.unwrap_unchecked()(glad::GL_FRAMEBUFFER, blit_fbo)
			})?;

			// Attach the Input texture (the shared texture) to the color buffer in our frame buffer - note texturetarget
			check_gl(|| {
				glad::glad_glFramebufferTexture2D.unwrap_unchecked()(
					glad::GL_READ_FRAMEBUFFER,
					glad::GL_COLOR_ATTACHMENT0,
					src_target,
					src_texture,
					0,
				)
			})?;
			check_gl(|| glad::glad_glReadBuffer.unwrap_unchecked()(glad::GL_COLOR_ATTACHMENT0))?;

			// Attach target texture (the one we write into and return) to second attachment point
			check_gl(|| {
				glad::glad_glFramebufferTexture2D.unwrap_unchecked()(
					glad::GL_DRAW_FRAMEBUFFER,
					glad::GL_COLOR_ATTACHMENT1,
					dst_target,
					dst_texture,
					0,
				)
			})?;

			check_gl(|| glad::glad_glDrawBuffer.unwrap_unchecked()(glad::GL_COLOR_ATTACHMENT1))?;

			// Check read/draw fbo for completeness
			match check_gl(|| {
				glad::glad_glCheckFramebufferStatus.unwrap_unchecked()(glad::GL_FRAMEBUFFER)
			})? {
				glad::GL_FRAMEBUFFER_COMPLETE => {
					if invert {
						// copy one texture buffer to the other while flipping upside down
						check_gl(|| {
							glad::glad_glBlitFramebuffer.unwrap_unchecked()(
								src_dimensions.top_left[0],     // srcX0
								src_dimensions.top_left[1],     // srcY0,
								src_dimensions.bottom_right[0], // srcX1
								src_dimensions.bottom_right[1], // srcY1
								dst_dimensions.top_left[0],     // dstX0
								dst_dimensions.bottom_right[1], // dstY0,
								dst_dimensions.bottom_right[0], // dstX1
								dst_dimensions.top_left[1],     // dstY1
								glad::GL_COLOR_BUFFER_BIT,
								glad::GL_LINEAR,
							)
						})?;
					} else {
						// Do not flip during blit
						check_gl(|| {
							glad::glad_glBlitFramebuffer.unwrap_unchecked()(
								src_dimensions.top_left[0],     // srcX0
								src_dimensions.top_left[1],     // srcY0,
								src_dimensions.bottom_right[0], // srcX1
								src_dimensions.bottom_right[1], // srcY1
								dst_dimensions.top_left[0],     // dstX0
								dst_dimensions.top_left[1],     // dstY0
								dst_dimensions.bottom_right[0], // dstX1
								dst_dimensions.bottom_right[1], // dstY1,
								glad::GL_COLOR_BUFFER_BIT,
								glad::GL_LINEAR,
							)
						})?;
					}
				}
				_ => {}
			}

			// restore the previous fbo - default is 0
			check_gl(|| glad::glad_glDrawBuffer.unwrap_unchecked()(glad::GL_COLOR_ATTACHMENT0))?;
			check_gl(|| {
				glad::glad_glBindFramebuffer.unwrap_unchecked()(glad::GL_FRAMEBUFFER, prev_fbo)
			})?;
		};

		Ok(blit_fbo)
	}

	pub fn recv_blit_image(
		&mut self,
		src_texture: glad::GLuint,
		src_target: glad::GLuint,
		src_dimensions: &GlImageExtent,
		invert: bool,
		prev_fbo: glad::GLuint,
	) -> Result<(), glad::GLuint> {
		let blit_fbo = Self::blit_image(
			src_texture,
			src_target,
			src_dimensions,
			self.texture,
			glad::GL_TEXTURE_2D,
			&GlImageExtent {
				top_left: [0, 0],
				bottom_right: [
					self.data.width as glad::GLint,
					self.data.height as glad::GLint,
				],
			},
			invert,
			self.fbo,
			prev_fbo,
		)?;
		self.fbo = blit_fbo;

		Ok(())
	}

	pub fn send_blit_image(
		&mut self,
		dst_texture: glad::GLuint,
		dst_target: glad::GLuint,
		dst_dimensions: &GlImageExtent,
		invert: bool,
		prev_fbo: glad::GLuint,
	) -> Result<(), glad::GLuint> {
		let blit_fbo = Self::blit_image(
			self.texture,
			glad::GL_TEXTURE_2D,
			&GlImageExtent {
				top_left: [0, 0],
				bottom_right: [
					self.data.width as glad::GLint,
					self.data.height as glad::GLint,
				],
			},
			dst_texture,
			dst_target,
			dst_dimensions,
			invert,
			self.fbo,
			prev_fbo,
		)?;
		self.fbo = blit_fbo;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::glad;
	use super::GlSharedImage;
	use glfw::fail_on_errors;
	use glfw::Context;

	#[allow(dead_code)]
	struct GlfwData<R> {
		glfw: glfw::Glfw,
		window: glfw::PWindow,
		events: glfw::GlfwReceiver<R>,
	}

	fn _init_gl() -> GlfwData<(f64, glfw::WindowEvent)> {
		let mut glfw = glfw::init(fail_on_errors!()).unwrap();
		glfw.window_hint(glfw::WindowHint::Visible(false));

		// Create a windowed mode window and its OpenGL context
		let (mut window, events) = glfw
			.create_window(300, 300, "Hello this is window", glfw::WindowMode::Windowed)
			.expect("Failed to create GLFW window.");
		window.make_current();

		GlSharedImage::init_gl().unwrap();

		GlfwData {
			glfw,
			window,
			events,
		}
	}

	#[test]
	fn gl_shared_image_init_gl() {
		_init_gl();
	}

	#[test]
	fn gl_shared_image_new() {
		let _gl_context = _init_gl();
		let _gl_shared_image =
			GlSharedImage::new(1, 1, 4, glad::GL_RGBA, glad::GL_RGBA as i32, 0).unwrap();
	}
}
