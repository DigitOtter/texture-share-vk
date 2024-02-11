use ash::vk;
use libc::c_void;
use std::ptr::NonNull;
use std::{alloc, ptr};

use super::vk_cpu_buffer::VkCpuBuffer;
use super::vk_shared_image::VkSharedImage;
use crate::vk_device::VkDevice;
use crate::vk_instance::VkInstance;
use crate::vk_shared_image::ImageBlit;

pub struct VkCpuSharedImage {
	pub image: VkSharedImage,
	pub cpu_buffer: VkCpuBuffer,
}

pub struct AlignedRamBuffer {
	pub layout: alloc::Layout,
	pub ptr: *mut c_void,
}

impl Drop for VkCpuSharedImage {
	fn drop(&mut self) {
		println!("Warning: VkCpuSharedImage should be manually destroyed, not dropped");
	}
}

impl Drop for AlignedRamBuffer {
	fn drop(&mut self) {
		if !self.ptr.is_null() {
			unsafe { alloc::dealloc(self.ptr as *mut _, self.layout) };
			self.ptr = ptr::null_mut();
		}
	}
}

impl Default for AlignedRamBuffer {
	fn default() -> Self {
		Self {
			layout: alloc::Layout::new::<u8>(),
			ptr: ptr::null_mut(),
		}
	}
}

impl AlignedRamBuffer {
	pub fn new(min_size: usize, align: usize) -> AlignedRamBuffer {
		let layout = alloc::Layout::from_size_align(min_size, align)
			.expect("Unable to create memory layout");
		let ptr = unsafe { alloc::alloc(layout) };

		AlignedRamBuffer {
			layout,
			ptr: ptr as *mut _,
		}
	}
}

impl VkCpuSharedImage {
	pub fn new(
		vk_instance: &VkInstance,
		vk_device: &VkDevice,
		width: u32,
		height: u32,
		format: vk::Format,
		id: u32,
	) -> Result<VkCpuSharedImage, vk::Result> {
		let vk_shared_image =
			VkSharedImage::new(vk_instance, vk_device, width, height, format, id)?;
		Self::from_shared_image(vk_instance, vk_device, vk_shared_image)
	}

	pub fn from_shared_image(
		vk_instance: &VkInstance,
		vk_device: &VkDevice,
		image: VkSharedImage,
	) -> Result<VkCpuSharedImage, vk::Result> {
		let cpu_buffer =
			VkCpuBuffer::new(vk_instance, vk_device, image.data.allocation_size, None)?;
		Ok(VkCpuSharedImage { image, cpu_buffer })
	}

	pub fn destroy(self, vk_device: &VkDevice) {
		self.cpu_buffer._destroy(vk_device);
		self.image._destroy(vk_device);

		std::mem::forget(self);
	}

	pub fn gen_device_aligned_ram_buffer(
		min_size: usize,
		vk_instance: &VkInstance,
		physical_device: vk::PhysicalDevice,
	) -> AlignedRamBuffer {
		let align =
			VkDevice::get_external_memory_host_properties(&vk_instance.instance, physical_device)
				.min_imported_host_pointer_alignment as usize;

		AlignedRamBuffer::new(min_size, align)
	}

	pub fn resize_image(
		&mut self,
		vk_instance: &VkInstance,
		vk_device: &VkDevice,
		width: u32,
		height: u32,
		format: vk::Format,
		id: u32,
		ram_buffer: &mut AlignedRamBuffer,
	) -> Result<(), vk::Result> {
		self.image
			.resize_image(vk_instance, vk_device, width, height, format, id)?;
		if self.image.data.allocation_size as usize > ram_buffer.layout.align() {
			*ram_buffer = VkCpuSharedImage::gen_device_aligned_ram_buffer(
				self.image.data.allocation_size as usize,
				vk_instance,
				vk_device.physical_device,
			);
		}

		self.cpu_buffer.resize(
			vk_instance,
			vk_device,
			ram_buffer.layout.align() as u64,
			Some(NonNull::new(ram_buffer.ptr).unwrap()),
		)?;

		Ok(())
	}

	// pub fn to_shared_image(self, vk_setup: &VkDevice) -> VkSharedImage {
	// 	self.cpu_buffer._destroy(vk_setup);
	// 	std::mem::forget(self);

	// 	self.image
	// }
}

impl ImageBlit for VkCpuSharedImage {
	fn send_image_blit_with_extents(
		&self,
		vk_device: &VkDevice,
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
			// 2. TRANSFER: Copy buffer to image
			// 3. TRANSFER: Blit image

			// 1. HOST
			let src_buf_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::HOST_READ,
				self.cpu_buffer.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[src_buf_barrier],
					&[],
				);
			}

			// 2. TRANSFER
			let src_buf_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::HOST_READ,
				vk::AccessFlags::TRANSFER_READ,
				self.cpu_buffer.buffer_size,
			);
			let src_image_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				self.image.image_layout,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[src_buf_barrier],
					&[src_image_barrier],
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
				vk_device.device.cmd_copy_buffer_to_image(
					cmd_bud,
					self.cpu_buffer.buffer.handle,
					self.image.image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[copy_region],
				)
			}

			// 3. TRANSFER
			let src_buf_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
				self.cpu_buffer.buffer_size,
			);
			let src_img_barrier = VkSharedImage::gen_img_mem_barrier(
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
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[src_buf_barrier],
					&[src_img_barrier, dst_img_barrier],
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
				vk_device.device.cmd_blit_image(
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

			let src_img_barrier = VkSharedImage::gen_img_mem_barrier(
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
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[],
					&[src_img_barrier, dst_img_barrier],
				);
			}

			Ok(())
		};

		self.cpu_buffer.sync_memory_from_cpu(vk_device)?;
		vk_device.immediate_submit_with_fence(
			vk_device.command_buffer,
			send_image_cmd_fcn,
			&[],
			&[],
			fence,
		)?;

		Ok(())
	}

	fn send_image_blit(
		&self,
		vk_device: &VkDevice,
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
			vk_device,
			dst_image,
			orig_dst_image_layout,
			target_dst_image_layout,
			&dst_image_extent,
			fence,
		)
	}

	fn recv_image_blit_with_extents(
		&self,
		vk_device: &VkDevice,
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

		let recv_image_cmd_fcn = |cmd_bud: vk::CommandBuffer| -> Result<(), vk::Result> {
			// Pipeline steps:
			// 1. TRANSFER: Blit image
			// 2. TRANSFER: Copy image to CPU buffer
			// 3. HOST: GPU -> CPU transfer for self.cpu_buffer

			let src_img_barrier = VkSharedImage::gen_img_mem_barrier(
				*src_image,
				orig_src_image_layout,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_READ,
			);
			let dst_img_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				self.image.image_layout,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[],
					&[src_img_barrier, dst_img_barrier],
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
				vk_device.device.cmd_blit_image(
					cmd_bud,
					*src_image,
					vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
					self.image.image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[vk::ImageBlit {
						src_offsets: *src_image_extent,
						src_subresource: image_subresource_layer,
						dst_offsets: dst_image_extent,
						dst_subresource: image_subresource_layer,
					}],
					vk::Filter::NEAREST,
				)
			}

			let src_img_barrier = VkSharedImage::gen_img_mem_barrier(
				*src_image,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				target_src_image_layout,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
			);
			let dst_img_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::TRANSFER_READ,
			);
			let dst_buffer_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
				self.cpu_buffer.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[dst_buffer_barrier],
					&[src_img_barrier, dst_img_barrier],
				);
			}

			// Copy image to buffer
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
				vk_device.device.cmd_copy_image_to_buffer(
					cmd_bud,
					self.image.image,
					vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
					self.cpu_buffer.buffer.handle,
					&[copy_region],
				)
			}

			let dst_img_barrier = VkSharedImage::gen_img_mem_barrier(
				self.image.image,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				self.image.image_layout,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
			);
			let dst_buffer_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::HOST_WRITE,
				self.cpu_buffer.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[dst_buffer_barrier],
					&[dst_img_barrier],
				);
			}

			let dst_buffer_barrier = VkCpuBuffer::gen_buffer_memory_barrier(
				self.cpu_buffer.buffer.handle,
				vk::AccessFlags::HOST_WRITE,
				vk::AccessFlags::NONE,
				self.cpu_buffer.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_bud,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[dst_buffer_barrier],
					&[],
				);
			}

			Ok(())
		};

		vk_device.immediate_submit_with_fence(
			vk_device.command_buffer,
			recv_image_cmd_fcn,
			&[],
			&[],
			fence,
		)?;
		self.cpu_buffer.sync_memory_to_cpu(vk_device)?;

		Ok(())
	}

	fn recv_image_blit(
		&self,
		vk_device: &VkDevice,
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
			vk_device,
			src_image,
			orig_src_image_layout,
			target_src_image_layout,
			&src_image_extent,
			fence,
		)
	}
}

#[cfg(test)]
mod tests {
	use std::ffi::CStr;
	use std::slice;

	use ash::vk;

	use crate::vk_device::VkDevice;
	use crate::vk_instance::VkInstance;
	use crate::vk_shared_image::ImageBlit;

	use super::VkCpuSharedImage;

	fn _init_vk_device() -> (VkInstance, VkDevice) {
		let vk_instance = VkInstance::new(
			None,
			CStr::from_bytes_with_nul(b"VkDevice\0").unwrap(),
			true,
		)
		.unwrap();
		let vk_device = VkDevice::new(&vk_instance, None).unwrap();
		(vk_instance, vk_device)
	}

	#[test]
	fn vk_cpu_shared_image_new() {
		let (vk_instance, vk_device) = _init_vk_device();
		let vk_cpu_shared_image = VkCpuSharedImage::new(
			&vk_instance,
			&vk_device,
			1,
			1,
			vk::Format::R8G8B8A8_UNORM,
			0,
		)
		.expect("Unable to create VkCpuSharedImage");

		vk_cpu_shared_image.destroy(&vk_device);
	}

	#[test]
	fn vk_cpu_shared_image_copy() {
		let (vk_instance, vk_device) = _init_vk_device();
		let vk_cpu_shared_image_in = VkCpuSharedImage::new(
			&vk_instance,
			&vk_device,
			1,
			1,
			vk::Format::R8G8B8A8_UNORM,
			0,
		)
		.expect("Unable to create vk_cpu_shared_image_in");

		let vk_cpu_shared_image_out = VkCpuSharedImage::new(
			&vk_instance,
			&vk_device,
			1,
			1,
			vk::Format::R8G8B8A8_UNORM,
			0,
		)
		.expect("Unable to create vk_cpu_shared_image_out");

		let ram_in = unsafe {
			slice::from_raw_parts_mut(
				vk_cpu_shared_image_in.cpu_buffer.ram_memory as *mut u8,
				vk_cpu_shared_image_in.cpu_buffer.buffer_size as usize,
			)
		};
		let ram_out = unsafe {
			slice::from_raw_parts_mut(
				vk_cpu_shared_image_out.cpu_buffer.ram_memory as *mut u8,
				vk_cpu_shared_image_out.cpu_buffer.buffer_size as usize,
			)
		};

		let fence = vk_device.create_fence(None).unwrap();

		let test_val = 5;
		let fake_val = 31;

		// Test recv_image_blit
		ram_in[0] = test_val;
		ram_out[0] = fake_val;

		vk_cpu_shared_image_in
			.cpu_buffer
			.write_image_from_cpu(
				&vk_device,
				vk_cpu_shared_image_in.image.image,
				vk_cpu_shared_image_in.image.image_layout,
				vk_cpu_shared_image_in.image.data.width,
				vk_cpu_shared_image_in.image.data.height,
			)
			.unwrap();

		vk_cpu_shared_image_out
			.recv_image_blit(
				&vk_device,
				&vk_cpu_shared_image_in.image.image,
				vk_cpu_shared_image_in.image.image_layout,
				vk_cpu_shared_image_in.image.image_layout,
				fence,
			)
			.expect("Unable to recv_image_blit");

		assert_eq!(ram_in[0], test_val);
		assert_eq!(ram_out[0], test_val);

		// Test send_image_blit
		ram_out[0] = fake_val;

		vk_cpu_shared_image_in
			.send_image_blit(
				&vk_device,
				&vk_cpu_shared_image_out.image.image,
				vk_cpu_shared_image_out.image.image_layout,
				vk_cpu_shared_image_out.image.image_layout,
				fence,
			)
			.expect("Unable to send_image_blit");

		vk_cpu_shared_image_out
			.cpu_buffer
			.read_image_to_cpu(
				&vk_device,
				vk_cpu_shared_image_out.image.image,
				vk_cpu_shared_image_out.image.image_layout,
				vk_cpu_shared_image_out.image.data.width,
				vk_cpu_shared_image_out.image.data.height,
			)
			.unwrap();

		assert_eq!(ram_in[0], test_val);
		assert_eq!(ram_out[0], test_val);

		vk_device.destroy_fence(fence);

		vk_cpu_shared_image_out.destroy(&vk_device);
		vk_cpu_shared_image_in.destroy(&vk_device);
	}
}
