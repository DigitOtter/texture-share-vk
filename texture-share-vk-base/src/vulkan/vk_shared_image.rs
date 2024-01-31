use std::os::fd::{AsRawFd, OwnedFd};

use ash::vk;
use texture_share_ipc::platform::{img_data::ImgFormat, ShmemDataInternal};

use crate::vk_setup::{VkCommandBuffera, VkSetup};

#[derive(Clone)]
#[repr(C)]
pub struct SharedImageData {
	pub id: u32,
	pub width: u32,
	pub height: u32,
	pub format: vk::Format,
	pub allocation_size: u64,
}

impl SharedImageData {
	pub fn from_shmem_img_data(data: &ShmemDataInternal) -> SharedImageData {
		SharedImageData {
			id: data.handle_id,
			width: data.width,
			height: data.height,
			format: vk::Format::R8G8B8A8_UNORM, //TODO: Change
			allocation_size: data.allocation_size,
		}
	}
}

pub struct VkSharedImage {
	pub image: vk::Image,
	pub image_layout: vk::ImageLayout,
	memory: vk::DeviceMemory,

	data: SharedImageData,
	//_phantom_dev: PhantomData<&'a VkSetup>,
}

#[cfg(target_os = "linux")]
type VkMemoryHandle = OwnedFd;

impl Drop for VkSharedImage {
	fn drop(&mut self) {
		if self.image_layout != vk::ImageLayout::UNDEFINED {
			println!("Warning: VkSharedImage should be manually destroyed, not dropped");
		}
	}
}

impl VkSharedImage {
	const DEFAULT_IMAGE_LAYOUT: vk::ImageLayout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;

	pub fn get_vk_format(format: ImgFormat) -> vk::Format {
		match format {
			ImgFormat::B8G8R8 => vk::Format::B8G8R8_UNORM,
			ImgFormat::B8G8R8A8 => vk::Format::B8G8R8A8_UNORM,
			ImgFormat::R8G8B8 => vk::Format::R8G8B8_UNORM,
			ImgFormat::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,
			ImgFormat::Undefined => vk::Format::UNDEFINED,
		}
	}

	pub fn get_img_format(format: vk::Format) -> ImgFormat {
		match format {
			vk::Format::B8G8R8_UNORM => ImgFormat::B8G8R8,
			vk::Format::B8G8R8A8_UNORM => ImgFormat::B8G8R8A8,
			vk::Format::R8G8B8_UNORM => ImgFormat::R8G8B8,
			vk::Format::R8G8B8A8_UNORM => ImgFormat::R8G8B8A8,
			vk::Format::UNDEFINED => ImgFormat::Undefined,
			_ => panic!("VkFormat {:?} not implemented", format),
		}
	}

	#[cfg(target_os = "linux")]
	const MEMORY_HANDLE_TYPE_FLAG: vk::ExternalMemoryHandleTypeFlags =
		vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;

	pub fn new(
		vk_setup: &VkSetup,
		width: u32,
		height: u32,
		format: vk::Format,
		id: u32,
	) -> Result<VkSharedImage, vk::Result> {
		// Allocate image memory
		let mut external_memory_image_info = vk::ExternalMemoryImageCreateInfo::builder()
			.handle_types(Self::MEMORY_HANDLE_TYPE_FLAG)
			.build();

		let image_create_info = vk::ImageCreateInfo::builder()
			.image_type(vk::ImageType::TYPE_2D)
			.format(format)
			.mip_levels(1)
			.array_layers(1)
			.samples(vk::SampleCountFlags::TYPE_1)
			.extent(vk::Extent3D {
				width,
				height,
				depth: 1,
				..Default::default()
			})
			.usage(
				vk::ImageUsageFlags::COLOR_ATTACHMENT
					| vk::ImageUsageFlags::SAMPLED
					| vk::ImageUsageFlags::TRANSFER_SRC
					| vk::ImageUsageFlags::TRANSFER_DST,
			)
			.push_next(&mut external_memory_image_info)
			.build();

		let image = unsafe { vk_setup.vk_device.create_image(&image_create_info, None) }?;

		let memory_requirements =
			unsafe { vk_setup.vk_device.get_image_memory_requirements(image) };
		let mut export_memory_alloc_info = vk::ExportMemoryAllocateInfo::builder()
			.handle_types(Self::MEMORY_HANDLE_TYPE_FLAG)
			.build();
		let mem_allocate_info = vk::MemoryAllocateInfo::builder()
			.allocation_size(memory_requirements.size)
			.memory_type_index(
				vk_setup
					.get_memory_type(
						memory_requirements.memory_type_bits,
						vk::MemoryPropertyFlags::DEVICE_LOCAL,
					)
					.unwrap(),
			)
			.push_next(&mut export_memory_alloc_info)
			.build();

		let memory = unsafe { vk_setup.vk_device.allocate_memory(&mem_allocate_info, None) }?;
		unsafe { vk_setup.vk_device.bind_image_memory(image, memory, 0) }?;

		// Initialize image
		let image_layout = Self::_set_image_layout(
			&image,
			vk_setup,
			vk::ImageLayout::UNDEFINED,
			Self::DEFAULT_IMAGE_LAYOUT,
			vk::AccessFlags::NONE,
			vk::AccessFlags::MEMORY_WRITE,
		)?;

		let data = SharedImageData {
			id,
			width,
			height,
			format,
			allocation_size: mem_allocate_info.allocation_size,
		};

		Ok(VkSharedImage {
			image,
			image_layout,
			memory,
			data,
			//_phantom_dev: PhantomData,
		})
	}

	pub fn resize_image(
		&mut self,
		vk_setup: &VkSetup,
		width: u32,
		height: u32,
		format: vk::Format,
		id: u32,
	) -> Result<(), vk::Result> {
		self._destroy(vk_setup);
		self.image_layout = vk::ImageLayout::UNDEFINED;
		*self = VkSharedImage::new(vk_setup, width, height, format, id)?;
		Ok(())
	}

	fn _destroy(&self, vk_setup: &VkSetup) {
		unsafe {
			vk_setup.vk_device.device_wait_idle().unwrap();
			vk_setup.vk_device.destroy_image(self.image, None);
			vk_setup.vk_device.free_memory(self.memory, None);
		}
	}

	pub fn destroy(self, vk_setup: &VkSetup) {
		self._destroy(&vk_setup);
		std::mem::forget(self)
	}

	pub fn import_from_handle(
		vk_setup: &VkSetup,
		mem_fd: VkMemoryHandle,
		image_data: SharedImageData,
	) -> Result<VkSharedImage, vk::Result> {
		// Create and allocate image memory
		let mut external_memory_image_info = vk::ExternalMemoryImageCreateInfo::builder()
			.handle_types(Self::MEMORY_HANDLE_TYPE_FLAG);
		let image_create_info = vk::ImageCreateInfo::builder()
			.push_next(&mut external_memory_image_info)
			.image_type(vk::ImageType::TYPE_2D)
			.format(vk::Format::R8G8B8A8_UNORM) // TODO: Use image_data.format
			.mip_levels(1)
			.array_layers(1)
			.samples(vk::SampleCountFlags::TYPE_1)
			.extent(vk::Extent3D {
				width: image_data.width,
				height: image_data.height,
				depth: 1,
				..Default::default()
			})
			.usage(
				vk::ImageUsageFlags::COLOR_ATTACHMENT
					| vk::ImageUsageFlags::SAMPLED
					| vk::ImageUsageFlags::TRANSFER_SRC
					| vk::ImageUsageFlags::TRANSFER_DST,
			)
			.build();

		let image = unsafe { vk_setup.vk_device.create_image(&image_create_info, None) }?;

		let memory_requirements =
			unsafe { vk_setup.vk_device.get_image_memory_requirements(image) };

		#[cfg(target_os = "linux")]
		let mut import_memory_info = vk::ImportMemoryFdInfoKHR::builder()
			.fd(mem_fd.as_raw_fd())
			.handle_type(Self::MEMORY_HANDLE_TYPE_FLAG)
			.build();

		let memory_allocate_info = vk::MemoryAllocateInfo::builder()
			.push_next(&mut import_memory_info)
			.allocation_size(memory_requirements.size)
			.memory_type_index(
				vk_setup
					.get_memory_type(
						memory_requirements.memory_type_bits,
						vk::MemoryPropertyFlags::DEVICE_LOCAL,
					)
					.unwrap(),
			)
			.build();

		let memory = unsafe {
			vk_setup
				.vk_device
				.allocate_memory(&memory_allocate_info, None)
		}?;

		// Handle ownership has been transferred to memory, release from mem_fd
		#[cfg(target_os = "linux")]
		std::mem::forget(mem_fd);

		unsafe { vk_setup.vk_device.bind_image_memory(image, memory, 0) }?;

		// Initialize image
		let image_layout = Self::_set_image_layout(
			&image,
			vk_setup,
			vk::ImageLayout::UNDEFINED,
			Self::DEFAULT_IMAGE_LAYOUT,
			vk::AccessFlags::NONE,
			vk::AccessFlags::MEMORY_WRITE,
		)?;

		Ok(VkSharedImage {
			image,
			image_layout,
			memory,
			data: image_data,
			//_phantom_dev: PhantomData,
		})
	}

	fn _set_image_layout(
		image: &vk::Image,
		vk_setup: &VkSetup,
		src_image_layout: vk::ImageLayout,
		dst_image_layout: vk::ImageLayout,
		src_access_mask: vk::AccessFlags,
		dst_access_mask: vk::AccessFlags,
	) -> Result<vk::ImageLayout, vk::Result> {
		// Initialize image
		let fence = vk_setup.create_fence(None)?;

		let image_layout_fcn = |com_buf: vk::CommandBuffer| {
			let img_mem_barrier = vk::ImageMemoryBarrier::builder()
				.image(*image)
				.src_access_mask(src_access_mask)
				.dst_access_mask(dst_access_mask)
				.old_layout(src_image_layout)
				.new_layout(dst_image_layout)
				.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.subresource_range(vk::ImageSubresourceRange {
					aspect_mask: vk::ImageAspectFlags::COLOR,
					level_count: 1,
					layer_count: 1,
					..Default::default()
				})
				.build();

			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					com_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::empty(),
					&[],
					&[],
					&[img_mem_barrier],
				)
			};

			Ok(())
		};
		vk_setup.immediate_submit(vk_setup.vk_command_buffer, image_layout_fcn, &[], &[])?;

		vk_setup.destroy_fence(fence);

		Ok(dst_image_layout)
	}

	pub fn get_image_data(&self) -> &SharedImageData {
		&self.data
	}

	#[cfg(target_os = "linux")]
	pub fn export_handle(&self, vk_setup: &VkSetup) -> Result<VkMemoryHandle, vk::Result> {
		use std::os::fd::FromRawFd;

		let memory_info = vk::MemoryGetFdInfoKHR::builder()
			.handle_type(Self::MEMORY_HANDLE_TYPE_FLAG)
			.memory(self.memory)
			.build();

		let fd = unsafe {
			OwnedFd::from_raw_fd(vk_setup.external_memory_fd.get_memory_fd(&memory_info)?)
		};

		Ok(fd)
	}

	pub fn set_image_layout(
		&mut self,
		vk_setup: &VkSetup,
		_vk_command_buffer: &VkCommandBuffera,
		src_image_layout: vk::ImageLayout,
		dst_image_layout: vk::ImageLayout,
		src_access_mask: vk::AccessFlags,
		dst_access_mask: vk::AccessFlags,
	) -> Result<vk::ImageLayout, vk::Result> {
		self.image_layout = Self::_set_image_layout(
			&self.image,
			vk_setup,
			src_image_layout,
			dst_image_layout,
			src_access_mask,
			dst_access_mask,
		)?;

		Ok(self.image_layout)
	}

	fn image_blit(
		vk_setup: &VkSetup,
		src_image: &vk::Image,
		orig_src_image_layout: vk::ImageLayout,
		target_src_image_layout: vk::ImageLayout,
		src_image_extent: &[vk::Offset3D; 2],
		dst_image: &vk::Image,
		orig_dst_image_layout: vk::ImageLayout,
		target_dst_image_layout: vk::ImageLayout,
		dst_image_extent: &[vk::Offset3D; 2],
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let blit_fcn = |cmd_buf: vk::CommandBuffer| -> Result<(), vk::Result> {
			let subresource_range: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
				aspect_mask: vk::ImageAspectFlags::COLOR,
				level_count: 1,
				layer_count: 1,
				..Default::default()
			};

			const SRC_BLIT_LAYOUT: vk::ImageLayout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
			const DST_BLIT_LAYOUT: vk::ImageLayout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;

			// Image memory barrier that prepares image transfer
			// Sets src_image to TRANSFER_SRC_OPTIMAL layout
			// Sets dst_image to TRANSFER_DST_OPTIMAL layout
			// Ensures that dst access masks are set to TRANSFER_READ and TRANSFER_WRITE respectively
			let src_img_mem_barrier = vk::ImageMemoryBarrier::builder()
				.image(*src_image)
				.src_access_mask(vk::AccessFlags::NONE)
				.dst_access_mask(vk::AccessFlags::TRANSFER_READ)
				.old_layout(orig_src_image_layout)
				.new_layout(SRC_BLIT_LAYOUT)
				.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.subresource_range(subresource_range)
				.build();
			let dst_img_mem_barrier = vk::ImageMemoryBarrier::builder()
				.image(*dst_image)
				.src_access_mask(vk::AccessFlags::NONE)
				.dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
				.old_layout(orig_dst_image_layout)
				.new_layout(DST_BLIT_LAYOUT)
				.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.subresource_range(subresource_range)
				.build();

			// Push pipeline barrier
			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TOP_OF_PIPE,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::default(),
					&[],
					&[],
					&[src_img_mem_barrier, dst_img_mem_barrier],
				)
			};

			// Blit image
			let image_subresource_layer = vk::ImageSubresourceLayers::builder()
				.aspect_mask(vk::ImageAspectFlags::COLOR)
				.base_array_layer(0)
				.layer_count(1)
				.mip_level(0)
				.build();
			let image_blit = vk::ImageBlit::builder()
				.src_subresource(image_subresource_layer)
				.src_offsets(*src_image_extent)
				.dst_subresource(image_subresource_layer)
				.dst_offsets(*dst_image_extent)
				.build();
			unsafe {
				vk_setup.vk_device.cmd_blit_image(
					cmd_buf,
					*src_image,
					SRC_BLIT_LAYOUT,
					*dst_image,
					DST_BLIT_LAYOUT,
					&[image_blit],
					vk::Filter::NEAREST,
				)
			};

			// Image memory barrier that waits for image transfer
			// Sets src_image to target_src_image_layout layout
			// Sets dst_image to target_dst_image_layout layout
			// Ensures that src access masks are set to TRANSFER_READ and TRANSFER_WRITE respectively
			let src_img_mem_barrier = vk::ImageMemoryBarrier::builder()
				.image(*src_image)
				.src_access_mask(vk::AccessFlags::TRANSFER_READ)
				.dst_access_mask(vk::AccessFlags::NONE)
				.old_layout(SRC_BLIT_LAYOUT)
				.new_layout(target_src_image_layout)
				.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.subresource_range(subresource_range)
				.build();
			let dst_img_mem_barrier = vk::ImageMemoryBarrier::builder()
				.image(*dst_image)
				.src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
				.dst_access_mask(vk::AccessFlags::NONE)
				.old_layout(DST_BLIT_LAYOUT)
				.new_layout(target_dst_image_layout)
				.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
				.subresource_range(subresource_range)
				.build();

			// Push pipeline barrier
			unsafe {
				vk_setup.vk_device.cmd_pipeline_barrier(
					cmd_buf,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::DependencyFlags::default(),
					&[],
					&[],
					&[src_img_mem_barrier, dst_img_mem_barrier],
				)
			};

			Ok(())
		};

		vk_setup.immediate_submit_with_fence(
			vk_setup.vk_command_buffer,
			blit_fcn,
			&[],
			&[],
			fence,
		)?;

		Ok(())
	}

	pub fn send_image_blit_with_extents(
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
				x: self.data.width as i32,
				y: self.data.height as i32,
				z: 1,
			},
		];

		Self::image_blit(
			vk_setup,
			&self.image,
			self.image_layout,
			self.image_layout,
			&src_image_extent,
			dst_image,
			orig_dst_image_layout,
			target_dst_image_layout,
			dst_image_extent,
			fence,
		)
	}

	pub fn send_image_blit(
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
				x: self.data.width as i32,
				y: self.data.height as i32,
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

	pub fn recv_image_blit_with_extents(
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
				x: self.data.width as i32,
				y: self.data.height as i32,
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
			self.image_layout,
			self.image_layout,
			&dst_image_extent,
			fence,
		)
	}

	pub fn recv_image_blit(
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
				x: self.data.width as i32,
				y: self.data.height as i32,
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

#[cfg(test)]
mod testsa {
	use std::ffi::CStr;

	use ash::vk;

	use crate::vk_setup::{VkCommandBuffera, VkCommandPoola, VkSetup};

	use super::VkSharedImage;

	fn _init_vk_setup<'a>() -> VkSetup {
		VkSetup::new(CStr::from_bytes_with_nul(b"VkSetup\0").unwrap()).unwrap()
	}

	fn _init_vk_command_pool<'a>(vk_setup: &'a VkSetup) -> VkCommandPoola<'a> {
		vk_setup.create_command_pool().unwrap()
	}

	fn _init_vk_command_buffer<'a>(
		command_pool: &'a VkCommandPoola<'_>,
		vk_setup: &VkSetup,
	) -> VkCommandBuffera<'a> {
		vk_setup
			.allocate_command_buffer(command_pool, vk::CommandBufferLevel::PRIMARY)
			.unwrap()
	}

	#[test]
	fn vk_shared_image_new() {
		let vk_setup = _init_vk_setup();

		let vk_shared_image =
			VkSharedImage::new(&vk_setup, 1, 1, vk::Format::R8G8B8A8_UNORM, 0).unwrap();

		vk_shared_image.destroy(&vk_setup);
	}

	#[test]
	fn vk_shared_image_export_handles() {
		let vk_setup = _init_vk_setup();

		let vk_shared_image =
			VkSharedImage::new(&vk_setup, 1, 1, vk::Format::R8G8B8A8_UNORM, 0).unwrap();

		let _shared_handle = vk_shared_image.export_handle(&vk_setup);
		std::mem::drop(_shared_handle);

		vk_shared_image.destroy(&vk_setup);
	}

	#[test]
	fn vk_shared_image_handle_exchange() {
		let vk_setup = _init_vk_setup();

		let width: u32 = 1;
		let height: u32 = 2;
		let format = vk::Format::R8G8B8A8_UNORM;
		let original_image = VkSharedImage::new(&vk_setup, width, height, format, 0).unwrap();

		let share_handle = original_image.export_handle(&vk_setup).unwrap();
		let import_img = VkSharedImage::import_from_handle(
			&vk_setup,
			share_handle,
			original_image.get_image_data().clone(),
		)
		.unwrap();

		import_img.destroy(&vk_setup);
		original_image.destroy(&vk_setup);
	}

	#[test]
	fn vk_shared_image_blit() {
		let vk_setup = _init_vk_setup();

		let width: u32 = 1;
		let height: u32 = 2;
		let format = vk::Format::R8G8B8A8_UNORM;
		let src_image = VkSharedImage::new(&vk_setup, width, height, format, 0).unwrap();
		let dst_image = VkSharedImage::new(&vk_setup, width, height, format, 0).unwrap();

		let fence = vk_setup.create_fence(None).unwrap();
		src_image
			.send_image_blit(
				&vk_setup,
				&dst_image.image,
				dst_image.image_layout,
				dst_image.image_layout,
				fence.handle,
			)
			.unwrap();
		vk_setup.destroy_fence(fence);

		dst_image.destroy(&vk_setup);
		src_image.destroy(&vk_setup);
	}
}

// #[cfg(test)]
// mod tests {
// 	use cxx::UniquePtr;

// 	use crate::vulkan::vk_setup::ffi::{vk_setup_new, VkSetup};

// 	use super::ffi::{vk_share_handles_new, vk_shared_image_new, VkFormat};

// 	fn _init_vulkan() -> UniquePtr<VkSetup> {
// 		let mut vk_setup = vk_setup_new();
// 		vk_setup.as_mut().unwrap().initialize_vulkan();

// 		vk_setup
// 	}

// 	#[test]
// 	fn vk_shared_image_share_handles_new() {
// 		let share_handles = vk_share_handles_new();
// 		assert_eq!(share_handles.get_memory_handle(), -1);
// 	}

// 	#[test]
// 	fn vk_shared_image_create() {
// 		let _ = vk_shared_image_new();
// 	}

// 	#[test]
// 	fn vk_shared_image_cleanup() {
// 		let mut vk_shared_image = vk_shared_image_new();
// 		vk_shared_image.as_mut().unwrap().cleanup();
// 	}

// 	#[test]
// 	fn vk_shared_image_data() {
// 		let mut vk_shared_image = vk_shared_image_new();
// 		const TEST_VAL: u32 = 12345;

// 		{
// 			let sh_dat = vk_shared_image.as_mut().unwrap().get_image_data_mut();
// 			sh_dat.id = TEST_VAL;
// 		}

// 		{
// 			let sh_dat = vk_shared_image.get_image_data();
// 			assert_eq!(sh_dat.id, TEST_VAL);
// 		}
// 	}

// 	#[test]
// 	fn vk_shared_image_init() {
// 		let mut vk_setup = vk_setup_new();
// 		vk_setup.as_mut().unwrap().initialize_vulkan();

// 		let _instance = vk_setup.as_ref().unwrap().get_vk_instance();
// 		let device = vk_setup.as_ref().unwrap().get_vk_device();
// 		let physical_device = vk_setup.as_ref().unwrap().get_vk_physical_device();
// 		// let queue = vk_setup.as_ref().unwrap().get_vk_queue();

// 		// initialize_vulkan_handles(
// 		//     instance,
// 		//     vk_setup.as_ref().unwrap().get_vk_physical_device(),
// 		// );

// 		let mut vk_shared_image = vk_shared_image_new();
// 		vk_shared_image.as_mut().unwrap().initialize(
// 			device,
// 			physical_device,
// 			vk_setup.get_vk_queue(),
// 			vk_setup.get_vk_command_buffer(),
// 			1,
// 			2,
// 			VkFormat::VK_FORMAT_R8G8B8A8_UNORM,
// 			3,
// 		);

// 		assert_eq!(vk_shared_image.get_image_data().width, 1);
// 		assert_eq!(vk_shared_image.get_image_data().height, 2);
// 		assert_eq!(
// 			vk_shared_image.get_image_data().format,
// 			VkFormat::VK_FORMAT_R8G8B8A8_UNORM
// 		);
// 		assert_eq!(vk_shared_image.get_image_data().id, 3);

// 		let _ = vk_shared_image
// 			.as_mut()
// 			.unwrap()
// 			.export_handles(vk_setup.get_external_handle_info());
// 	}

// 	#[test]
// 	fn vk_shared_image_handle_exchange() {
// 		let vk_setup = _init_vulkan();

// 		let mut original_img = vk_shared_image_new();

// 		let width: u32 = 1;
// 		let height: u32 = 2;
// 		let format = VkFormat::VK_FORMAT_R8G8B8A8_UNORM;
// 		original_img.as_mut().unwrap().initialize(
// 			vk_setup.get_vk_device(),
// 			vk_setup.get_vk_physical_device(),
// 			vk_setup.get_vk_queue(),
// 			vk_setup.get_vk_command_buffer(),
// 			width,
// 			height,
// 			format,
// 			0,
// 		);

// 		let share_handles = original_img
// 			.as_mut()
// 			.unwrap()
// 			.export_handles(vk_setup.get_external_handle_info());

// 		let mut import_img = vk_shared_image_new();
// 		let image_data = original_img.get_image_data();
// 		import_img.as_mut().unwrap().import_from_handle(
// 			vk_setup.get_vk_device(),
// 			vk_setup.get_vk_physical_device(),
// 			vk_setup.get_vk_queue(),
// 			vk_setup.get_vk_command_buffer(),
// 			share_handles,
// 			image_data,
// 		);
// 	}

// 	// #[test]
// 	// fn vk_shared_image_bridge_data() {
// 	//     let vk_shared_image = vk_shared_image_new();
// 	//     unsafe { vk_shared_image.as_ref().unwrap().ImageData() };
// 	// }
// }
