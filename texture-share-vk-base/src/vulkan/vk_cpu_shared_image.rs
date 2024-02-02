use ash::vk;

use super::vk_cpu_buffer::VkCpuBuffer;
use super::vk_shared_image::VkSharedImage;
use crate::vk_setup::VkSetup;
use crate::vk_shared_image::ImageBlit;

struct VkCpuSharedImage {
	image: VkSharedImage,
	cpu_buffer: VkCpuBuffer,
}

impl Drop for VkCpuSharedImage {
	fn drop(&mut self) {
		println!("Warning: VkCpuSharedImage should be manually destroyed, not dropped");
	}
}

impl VkCpuSharedImage {
	pub fn from_shared_image(
		vk_setup: &VkSetup,
		image: VkSharedImage,
	) -> Result<VkCpuSharedImage, vk::Result> {
		let cpu_buffer = VkCpuBuffer::new(vk_setup, image.data.allocation_size)?;
		Ok(VkCpuSharedImage { image, cpu_buffer })
	}

	pub fn destroy(self, vk_setup: &VkSetup) {
		self.cpu_buffer._destroy(vk_setup);
		self.image._destroy(vk_setup);

		std::mem::forget(self);
	}

	// pub fn to_shared_image(self, vk_setup: &VkSetup) -> VkSharedImage {
	// 	self.cpu_buffer._destroy(vk_setup);
	// 	std::mem::forget(self);

	// 	self.image
	// }
}

impl ImageBlit for VkCpuSharedImage {
	fn send_image_blit_with_extents(
		&self,
		vk_setup: &VkSetup,
		dst_image: &vk::Image,
		orig_dst_image_layout: vk::ImageLayout,
		target_dst_image_layout: vk::ImageLayout,
		dst_image_extent: &[vk::Offset3D; 2],
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let src_image_extent = [
			vk::Offset3D { x: 0, y: 0, z: 0 },
			vk::Offset3D {
				x: self.image.data.width as i32,
				y: self.image.data.height as i32,
				z: 1,
			},
		];

		let send_image_cmd_fcn = |cmd_bud: vk::CommandBuffer| -> Result<(), vk::Result> {
			// Pipeline steps:
			// 1. HOST: CPU -> GPU buffer transfer for self.cpu_buffer
			// 2. TRANSFER: Blit image

			// 1. HOST
			let src_cpu_buf_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::HOST_READ,
				vk::AccessFlags::TRANSFER_WRITE,
				self.cpu_buffer.buffer_size,
			);
			let dst_image_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				self.image.image_layout,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_READ,
			);
			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[src_cpu_buf_barrier],
					&[dst_image_barrier],
				);
			}

			// Copy buffer to image
			unsafe {
				let copy_region = vk::BufferImageCopy::builder()
					.buffer_row_length(0)
					.buffer_image_height(0)
					.image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
					.image_extent(vk::Extent3D {
						width: self.image.data.width,
						height: self.image.data.height,
						depth: 1,
					})
					.image_subresource(vk::ImageSubresourceLayers {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						base_array_layer: 0,
						layer_count: 1,
						mip_level: 0,
						..Default::default()
					})
					.build();
				vk_setup.vk_device.cmd_copy_buffer_to_image(
					cmd_bud,
					self.cpu_buffer.buffer.handle,
					self.image.image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[copy_region],
				)
			}

			let src_cpu_buf_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
				self.cpu_buffer.buffer_size,
			);
			let src_image_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::TRANSFER_READ,
			);
			let dst_img_barrier = VkSharedImage::gen_img_mem_barrier(
				*dst_image,
				orig_dst_image_layout,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
			);
			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[src_cpu_buf_barrier],
					&[src_image_barrier, dst_image_barrier],
				);
			}

			// Blit image
			unsafe {
				let image_subresource_layer = vk::ImageSubresourceLayers::builder()
					.aspect_mask(vk::ImageAspectFlags::COLOR)
					.base_array_layer(0)
					.layer_count(1)
					.mip_level(0)
					.build();
				vk_setup.vk_device.cmd_blit_image(
					cmd_bud,
					self.image.image,
					vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
					*dst_image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[vk::ImageBlit {
						src_offsets: src_image_extent,
						src_subresource: image_subresource_layer,
						dst_offsets: *dst_image_extent,
						dst_subresource: image_subresource_layer,
					}],
					vk::Filter::NEAREST,
				)
			}

			let src_image_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				self.image.image_layout,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
			);
			let dst_img_barrier = VkSharedImage::gen_img_mem_barrier(
				*dst_image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				target_dst_image_layout,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::NONE,
			);
			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[],
					&[src_image_barrier, dst_image_barrier],
				);
			}

			Ok(())
		};

		self.cpu_buffer.sync_memory_from_cpu(vk_setup)?;
		vk_setup.immediate_submit(vk_setup.vk_command_buffer, send_image_cmd_fcn, &[], &[])?;

		Ok(())
	}

	fn send_image_blit(
		&self,
		vk_setup: &VkSetup,
		dst_image: &vk::Image,
		orig_dst_image_layout: vk::ImageLayout,
		target_dst_image_layout: vk::ImageLayout,
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let dst_image_extent = [
			vk::Offset3D { x: 0, y: 0, z: 0 },
			vk::Offset3D {
				x: self.image.data.width as i32,
				y: self.image.data.height as i32,
				z: 1,
			},
		];

		self.send_image_blit_with_extents(
			vk_setup,
			dst_image,
			orig_dst_image_layout,
			target_dst_image_layout,
			&dst_image_extent,
			fence,
		)
	}

	fn recv_image_blit_with_extents(
		&self,
		vk_setup: &VkSetup,
		src_image: &vk::Image,
		orig_src_image_layout: vk::ImageLayout,
		target_src_image_layout: vk::ImageLayout,
		src_image_extent: &[vk::Offset3D; 2],
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let dst_image_extent = [
			vk::Offset3D { x: 0, y: 0, z: 0 },
			vk::Offset3D {
				x: self.image.data.width as i32,
				y: self.image.data.height as i32,
				z: 1,
			},
		];

		Self::image_blit(
			vk_setup,
			src_image,
			orig_src_image_layout,
			target_src_image_layout,
			&src_image_extent,
			&self.image,
			self.image.image_layout,
			self.image.image_layout,
			&dst_image_extent,
			fence,
		)
	}

	fn recv_image_blit(
		&self,
		vk_setup: &VkSetup,
		src_image: &vk::Image,
		orig_src_image_layout: vk::ImageLayout,
		target_src_image_layout: vk::ImageLayout,
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let src_image_extent = [
			vk::Offset3D { x: 0, y: 0, z: 0 },
			vk::Offset3D {
				x: self.image.data.width as i32,
				y: self.image.data.height as i32,
				z: 1,
			},
		];

		self.recv_image_blit_with_extents(
			vk_setup,
			src_image,
			orig_src_image_layout,
			target_src_image_layout,
			&src_image_extent,
			fence,
		)
	}
}
