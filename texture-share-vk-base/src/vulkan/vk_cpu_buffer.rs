use std::{
	borrow::{Borrow},
	os::raw::c_void,
	ptr::{self, NonNull},
};

use ash::vk::{self};

use crate::{
	vk_device::{VkBuffer, VkDevice},
	vk_instance::VkInstance,
	vk_shared_image::VkSharedImage,
};

pub struct VkCpuBuffer {
	pub buffer: VkBuffer,
	memory: vk::DeviceMemory,
	pub buffer_size: u64,
	pub ram_memory: *mut c_void,
}

impl Drop for VkCpuBuffer {
	fn drop(&mut self) {
		#[cfg(debug_assertions)]
		if !self.ram_memory.is_null() {
			println!("Warning: VkCpuBuffer should be manually destroyed, not dropped");
		}
	}
}

impl VkCpuBuffer {
	pub fn new(
		vk_instance: &VkInstance,
		vk_device: &VkDevice,
		buffer_size: u64,
		ram_memory: Option<NonNull<c_void>>,
	) -> Result<VkCpuBuffer, vk::Result> {
		let mut external_memory_buffer_info = vk::ExternalMemoryBufferCreateInfo::builder()
			.handle_types(vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT)
			.build();
		let create_info = vk::BufferCreateInfo::builder()
			.flags(vk::BufferCreateFlags::default())
			.size(buffer_size)
			.usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC)
			.queue_family_indices(&[vk_device.graphics_queue_family_index])
			.sharing_mode(vk::SharingMode::EXCLUSIVE)
			.push_next(&mut external_memory_buffer_info)
			.build();
		let buffer = vk_device.create_buffer(&create_info)?;

		let buffer_memory_requirements = unsafe {
			vk_device
				.device
				.get_buffer_memory_requirements(buffer.handle)
		};

		let mut memory_allocate_info = vk::MemoryAllocateInfo::builder()
			.allocation_size(buffer_size)
			.memory_type_index(
				vk_instance
					.get_memory_type(
						vk_device.physical_device,
						buffer_memory_requirements.memory_type_bits,
						vk::MemoryPropertyFlags::HOST_VISIBLE
							| vk::MemoryPropertyFlags::HOST_CACHED,
					)
					.unwrap(),
			);

		// Keep value, as a pointer to import_memory_info is used in memory_allocate_info
		let import_memory_info = if ram_memory.is_some() {
			// Ensure that vulkan allocates host memory at the specified location
			Some(
				vk::ImportMemoryHostPointerInfoEXT::builder()
					.handle_type(vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT)
					.host_pointer(ram_memory.as_ref().unwrap().as_ptr())
					.build(),
			)
		} else {
			None
		};

		if import_memory_info.is_some() {
			let ptr = import_memory_info.borrow().as_ref().unwrap();
			memory_allocate_info.p_next =
				ptr as *const vk::ImportMemoryHostPointerInfoEXT as *const _;
		}

		let memory = unsafe {
			vk_device
				.device
				.allocate_memory(&memory_allocate_info.build(), None)
		}?;

		unsafe {
			vk_device
				.device
				.bind_buffer_memory(buffer.handle, memory, 0)
		}?;

		// Map memory to RAM
		let mapped_ram_memory = unsafe {
			vk_device
				.device
				.map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::default())
		}?;

		// map_memory is only guaranteed to return the correct pointer if we don't explicitly state the ram region
		let ram_memory = match ram_memory {
			Some(ptr) => ptr.as_ptr(),
			None => mapped_ram_memory,
		};

		Ok(VkCpuBuffer {
			buffer,
			memory,
			buffer_size,
			ram_memory,
		})
	}

	pub(crate) fn _destroy(&self, vk_device: &VkDevice) {
		unsafe {
			vk_device.device.device_wait_idle().unwrap();

			vk_device.device.unmap_memory(self.memory);

			vk_device.device.destroy_buffer(self.buffer.handle, None);
			vk_device.device.free_memory(self.memory, None);
		}
	}

	pub fn destroy(self, vk_device: &VkDevice) {
		self._destroy(vk_device);
		std::mem::forget(self)
	}

	pub fn resize(
		&mut self,
		vk_instance: &VkInstance,
		vk_device: &VkDevice,
		buffer_size: u64,
		ram_memory: Option<NonNull<c_void>>,
	) -> Result<(), vk::Result> {
		self._destroy(vk_device);

		self.ram_memory = ptr::null_mut();
		*self = VkCpuBuffer::new(vk_instance, vk_device, buffer_size, ram_memory)?;

		Ok(())
	}

	pub fn gen_buffer_memory_barrier(
		buffer: vk::Buffer,
		src_access_mask: vk::AccessFlags,
		dst_access_mask: vk::AccessFlags,
		buffer_size: u64,
	) -> vk::BufferMemoryBarrier {
		vk::BufferMemoryBarrier::builder()
			.buffer(buffer)
			.src_access_mask(src_access_mask)
			.dst_access_mask(dst_access_mask)
			.offset(0)
			.size(buffer_size)
			.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
			.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
			.build()
	}

	pub fn read_image_to_cpu(
		&self,
		vk_device: &VkDevice,
		image: vk::Image,
		image_layout: vk::ImageLayout,
		image_width: u32,
		image_height: u32,
	) -> Result<(), vk::Result> {
		let img_copy_fcn = |cmd_buf: vk::CommandBuffer| {
			// Ensure that image is ready to send and buffer is ready for receive
			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
				self.buffer_size,
			);
			let img_mem_barrier = VkSharedImage::gen_img_mem_barrier(
				image,
				image_layout,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_READ,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[img_mem_barrier],
				)
			};

			// Setting buffer_row_length and buffer_image_height to 0 indicates a tightly packed memory range, with size determined by image_extent
			let copy_region = vk::BufferImageCopy::builder()
				.buffer_row_length(0)
				.buffer_image_height(0)
				.image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
				.image_extent(vk::Extent3D {
					width: image_width,
					height: image_height,
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

			unsafe {
				vk_device.device.cmd_copy_image_to_buffer(
					cmd_buf,
					image,
					vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
					self.buffer.handle,
					&[copy_region],
				)
			};

			// Ensure that memory write has been completed
			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::HOST_WRITE,
				self.buffer_size,
			);
			let img_mem_barrier = VkSharedImage::gen_img_mem_barrier(
				image,
				vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
				image_layout,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[img_mem_barrier],
				)
			};

			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::HOST_WRITE,
				vk::AccessFlags::NONE,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[],
				)
			};

			Ok(())
		};

		vk_device.immediate_submit(vk_device.command_buffer, img_copy_fcn, &[], &[])?;
		self.sync_memory_to_cpu(vk_device)?;

		Ok(())
	}

	pub fn write_image_from_cpu(
		&self,
		vk_device: &VkDevice,
		image: vk::Image,
		image_layout: vk::ImageLayout,
		image_width: u32,
		image_height: u32,
	) -> Result<(), vk::Result> {
		let img_copy_fcn = |cmd_buf: vk::CommandBuffer| {
			// Read from host. Not sure if this is required, but it works so I'll keep it
			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::HOST_READ,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[],
				)
			};

			// Ensure that buffer is ready to send and image is ready for receive
			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::HOST_READ,
				vk::AccessFlags::TRANSFER_READ,
				self.buffer_size,
			);
			let img_mem_barrier = VkSharedImage::gen_img_mem_barrier(
				image,
				image_layout,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				vk::AccessFlags::NONE,
				vk::AccessFlags::TRANSFER_WRITE,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[img_mem_barrier],
				)
			};

			// Setting buffer_row_length and buffer_image_height to 0 indicates a tightly packed memory range, with size determined by image_extent
			let copy_region = vk::BufferImageCopy::builder()
				.buffer_row_length(0)
				.buffer_image_height(0)
				.image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
				.image_extent(vk::Extent3D {
					width: image_width,
					height: image_height,
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

			unsafe {
				vk_device.device.cmd_copy_buffer_to_image(
					cmd_buf,
					self.buffer.handle,
					image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[copy_region],
				)
			};

			// Ensure that memory write has been completed
			let buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::TRANSFER_READ,
				vk::AccessFlags::NONE,
				self.buffer_size,
			);
			let img_mem_barrier = VkSharedImage::gen_img_mem_barrier(
				image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				image_layout,
				vk::AccessFlags::TRANSFER_WRITE,
				vk::AccessFlags::NONE,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[buf_mem_barrier],
					&[img_mem_barrier],
				)
			};

			Ok(())
		};

		self.sync_memory_from_cpu(vk_device)?;
		vk_device.immediate_submit(vk_device.command_buffer, img_copy_fcn, &[], &[])?;

		Ok(())
	}

	pub fn read_from_buffer(
		&self,
		vk_device: &VkDevice,
		read_buffer: vk::Buffer,
		read_memory: vk::DeviceMemory,
	) -> Result<(), vk::Result> {
		let buffer_read_fcn = |cmd_buf: vk::CommandBuffer| {
			// Ensure that buffers are ready
			let read_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				read_buffer,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_READ,
				self.buffer_size,
			);
			let write_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_WRITE,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[read_buf_mem_barrier, write_buf_mem_barrier],
					&[],
				)
			};

			// Copy buffer
			let region = vk::BufferCopy::builder()
				.src_offset(0)
				.dst_offset(0)
				.size(self.buffer_size)
				.build();
			unsafe {
				vk_device.device.cmd_copy_buffer(
					cmd_buf,
					read_buffer,
					self.buffer.handle,
					&[region],
				)
			};

			// Ensure that memory write has been completed
			let read_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				read_buffer,
				vk::AccessFlags::MEMORY_READ,
				vk::AccessFlags::NONE,
				self.buffer_size,
			);
			let write_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::MEMORY_WRITE,
				vk::AccessFlags::NONE,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[read_buf_mem_barrier, write_buf_mem_barrier],
					&[],
				)
			};

			Ok(())
		};

		Self::_sync_memory_from_cpu(read_memory, self.buffer_size, vk_device)?;
		vk_device.immediate_submit(vk_device.command_buffer, buffer_read_fcn, &[], &[])?;
		self.sync_memory_to_cpu(vk_device)?;

		Ok(())
	}

	pub fn write_to_buffer(
		&self,
		vk_device: &VkDevice,
		write_buffer: vk::Buffer,
		write_memory: vk::DeviceMemory,
	) -> Result<(), vk::Result> {
		let buffer_read_fcn = |cmd_buf: vk::CommandBuffer| {
			// Ensure that buffers are ready
			let read_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_READ,
				self.buffer_size,
			);
			let write_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				write_buffer,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_WRITE,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::HOST,
					vk::DependencyFlags::default(),
					&[],
					&[read_buf_mem_barrier, write_buf_mem_barrier],
					&[],
				)
			};

			// Copy buffer
			let region = vk::BufferCopy::builder()
				.src_offset(0)
				.dst_offset(0)
				.size(self.buffer_size)
				.build();
			unsafe {
				vk_device.device.cmd_copy_buffer(
					cmd_buf,
					self.buffer.handle,
					write_buffer,
					&[region],
				)
			};

			// Ensure that memory write has been completed
			let read_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				self.buffer.handle,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_READ,
				self.buffer_size,
			);
			let write_buf_mem_barrier = Self::gen_buffer_memory_barrier(
				write_buffer,
				vk::AccessFlags::NONE,
				vk::AccessFlags::MEMORY_WRITE,
				self.buffer_size,
			);
			unsafe {
				vk_device.device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::HOST,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[read_buf_mem_barrier, write_buf_mem_barrier],
					&[],
				)
			};

			Ok(())
		};

		self.sync_memory_from_cpu(vk_device)?;
		vk_device.immediate_submit(vk_device.command_buffer, buffer_read_fcn, &[], &[])?;
		Self::_sync_memory_to_cpu(write_memory, self.buffer_size, vk_device)?;

		Ok(())
	}

	fn _sync_memory_to_cpu(
		memory: vk::DeviceMemory,
		memory_size: u64,
		vk_device: &VkDevice,
	) -> Result<(), vk::Result> {
		unsafe {
			vk_device
				.device
				.invalidate_mapped_memory_ranges(&[vk::MappedMemoryRange::builder()
					.memory(memory)
					.size(memory_size)
					.build()])
		}
	}

	pub fn sync_memory_to_cpu(&self, vk_device: &VkDevice) -> Result<(), vk::Result> {
		Self::_sync_memory_to_cpu(self.memory, self.buffer_size, vk_device)?;
		Ok(())
	}

	fn _sync_memory_from_cpu(
		memory: vk::DeviceMemory,
		memory_size: u64,
		vk_device: &VkDevice,
	) -> Result<(), vk::Result> {
		unsafe {
			vk_device
				.device
				.flush_mapped_memory_ranges(&[vk::MappedMemoryRange::builder()
					.memory(memory)
					.size(memory_size)
					.build()])
		}
	}

	pub fn sync_memory_from_cpu(&self, vk_device: &VkDevice) -> Result<(), vk::Result> {
		Self::_sync_memory_from_cpu(self.memory, self.buffer_size, vk_device)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::{ffi::CStr, ptr::NonNull, slice};

	use ash::vk;

	use super::VkCpuBuffer;
	use crate::{vk_device::VkDevice, vk_instance::VkInstance, vk_shared_image::VkSharedImage};

	fn _init_vk_device() -> (VkInstance, VkDevice) {
		let vk_instance = VkInstance::new(
			None,
			CStr::from_bytes_with_nul(b"VkDevice\0").unwrap(),
			false,
		)
		.unwrap();
		let vk_device = VkDevice::new(&vk_instance, None).unwrap();
		(vk_instance, vk_device)
	}

	#[test]
	fn vk_cpu_buffer_new() {
		let (vk_instance, vk_device) = _init_vk_device();
		let vk_cpu_buffer = VkCpuBuffer::new(&vk_instance, &vk_device, 4, None)
			.expect("Unable to initialize VkCpuBuffer");

		vk_cpu_buffer.destroy(&vk_device);
	}

	#[test]
	fn vk_cpu_buffer_new_preallocated_ram() {
		const BUFFER_SIZE: usize = 4;
		let (vk_instance, vk_device) = _init_vk_device();

		let host_mem_props = VkDevice::get_external_memory_host_properties(
			&vk_instance.instance,
			vk_device.physical_device,
		);

		let layout = std::alloc::Layout::from_size_align(
			BUFFER_SIZE,
			host_mem_props.min_imported_host_pointer_alignment as usize,
		)
		.unwrap();
		let ram_buffer = unsafe { std::alloc::alloc(layout) };

		let vk_cpu_buffer = VkCpuBuffer::new(
			&vk_instance,
			&vk_device,
			layout.align() as u64,
			Some(NonNull::new(ram_buffer as *mut _).unwrap()),
		)
		.expect("Unable to initialize VkCpuBuffer");

		vk_cpu_buffer.destroy(&vk_device);

		unsafe { std::alloc::dealloc(ram_buffer, layout) };
	}

	#[test]
	fn vk_cpu_buffer_buffer_read_write() {
		let buffer_size: u64 = 4;

		let (vk_instance, vk_device) = _init_vk_device();
		let vk_cpu_buffer_in = VkCpuBuffer::new(&vk_instance, &vk_device, buffer_size, None)
			.expect("Unable to initialize vk_cpu_buffer_in");
		let vk_cpu_buffer_out = VkCpuBuffer::new(&vk_instance, &vk_device, buffer_size, None)
			.expect("Unable to initialize vk_cpu_buffer_in");

		let in_buffer = unsafe {
			slice::from_raw_parts_mut(vk_cpu_buffer_in.ram_memory as *mut u8, buffer_size as usize)
		};

		let out_buffer = unsafe {
			slice::from_raw_parts_mut(
				vk_cpu_buffer_out.ram_memory as *mut u8,
				buffer_size as usize,
			)
		};

		let test_val = 34;
		let fake_val = 10;

		// Test write
		in_buffer[0] = test_val;
		out_buffer[0] = fake_val;

		vk_cpu_buffer_in
			.write_to_buffer(
				&vk_device,
				vk_cpu_buffer_out.buffer.handle,
				vk_cpu_buffer_out.memory,
			)
			.expect("Failed to write buffer");

		assert_eq!(in_buffer[0], test_val);
		assert_eq!(out_buffer[0], test_val);

		// Test write
		in_buffer[0] = test_val;
		out_buffer[0] = fake_val;

		vk_cpu_buffer_out
			.read_from_buffer(
				&vk_device,
				vk_cpu_buffer_in.buffer.handle,
				vk_cpu_buffer_in.memory,
			)
			.expect("Failed to read buffer");

		assert_eq!(in_buffer[0], test_val);
		assert_eq!(out_buffer[0], test_val);

		vk_cpu_buffer_out.destroy(&vk_device);
		vk_cpu_buffer_in.destroy(&vk_device);
	}

	#[test]
	fn vk_cpu_buffer_image_read_write() {
		let (vk_instance, vk_device) = _init_vk_device();

		let vk_shared_image = VkSharedImage::new(
			&vk_instance,
			&vk_device,
			1,
			1,
			vk::Format::R8G8B8A8_UNORM,
			0,
		)
		.expect("Unable to create VkSharedImage");

		let vk_cpu_buffer_in = VkCpuBuffer::new(
			&vk_instance,
			&vk_device,
			vk_shared_image.data.allocation_size,
			None,
		)
		.expect("Unable to initialize vk_cpu_buffer_in");
		let vk_cpu_buffer_out = VkCpuBuffer::new(
			&vk_instance,
			&vk_device,
			vk_shared_image.data.allocation_size,
			None,
		)
		.expect("Unable to initialize vk_cpu_buffer_in");

		let in_buffer = unsafe {
			slice::from_raw_parts_mut(
				vk_cpu_buffer_in.ram_memory as *mut u8,
				vk_shared_image.data.allocation_size as usize,
			)
		};

		let out_buffer = unsafe {
			slice::from_raw_parts_mut(
				vk_cpu_buffer_out.ram_memory as *mut u8,
				vk_shared_image.data.allocation_size as usize,
			)
		};

		let test_val = 34;
		let fake_val = 10;

		// Test read/write
		in_buffer[0] = test_val;
		out_buffer[0] = fake_val;

		vk_cpu_buffer_in
			.write_image_from_cpu(
				&vk_device,
				vk_shared_image.image,
				vk_shared_image.image_layout,
				vk_shared_image.data.width,
				vk_shared_image.data.height,
			)
			.expect("Failed to write image");

		vk_cpu_buffer_out
			.read_image_to_cpu(
				&vk_device,
				vk_shared_image.image,
				vk_shared_image.image_layout,
				vk_shared_image.data.width,
				vk_shared_image.data.height,
			)
			.expect("Failed to read image");

		assert_eq!(in_buffer[0], test_val);
		assert_eq!(out_buffer[0], test_val);

		vk_cpu_buffer_out.destroy(&vk_device);
		vk_cpu_buffer_in.destroy(&vk_device);
		vk_shared_image.destroy(&vk_device);
	}
}
