use ash::{vk, Device, Instance};
use std::ffi::{CStr, CString};
use texture_share_ipc::uuid;

use super::vk_entry::VkEntry;
use super::vk_instance::VkInstance;

pub struct VkDevice {
	pub device: Device,
	pub physical_device: vk::PhysicalDevice,

	pub graphics_queue: vk::Queue,
	pub graphics_queue_family_index: u32,
	pub graphics_queue_index: u32,

	import_only: bool,

	pub command_pool: vk::CommandPool,
	pub command_buffer: vk::CommandBuffer,

	#[cfg(target_os = "linux")]
	pub external_memory_fd: ash::extensions::khr::ExternalMemoryFd,
}

pub struct VkPhysicalDeviceOptions {
	pub vendor_id: Option<u32>,
	pub device_id: Option<u32>,
	pub device_uuid: Option<uuid::Uuid>,
	pub device_name: Option<CString>,
	pub device_type: Option<vk::PhysicalDeviceType>,
}

enum ExtensionOptions<'a> {
	Name(&'a CStr),
	SelectFirst(Vec<&'a CStr>),
}

pub struct VkBuffer {
	pub handle: vk::Buffer,
}

impl Drop for VkDevice {
	fn drop(&mut self) {
		unsafe {
			self.device
				.device_wait_idle()
				.expect("Unable to wait for device idle");

			// CommandPool and CommandBuffer are always owned by VkDevice
			self._free_command_buffer(&self.command_pool, self.command_buffer);
			self._destroy_command_pool(self.command_pool);

			if !self.import_only {
				self.device.destroy_device(None);
			};
		}
	}
}

impl Drop for VkBuffer {
	fn drop(&mut self) {
		//println!("Warning: VkBuffer should be manually destroyed, not dropped");
	}
}

impl Default for VkPhysicalDeviceOptions {
	fn default() -> Self {
		VkPhysicalDeviceOptions {
			vendor_id: None,
			device_id: None,
			device_uuid: None,
			device_name: None,
			device_type: None,
		}
	}
}

impl VkDevice {
	pub fn new(
		vk_instance: &VkInstance,
		physical_device_options: Option<VkPhysicalDeviceOptions>,
	) -> Result<VkDevice, vk::Result> {
		let extensions = [
			ExtensionOptions::Name(vk::KhrExternalSemaphoreFn::name()),
			ExtensionOptions::Name(vk::KhrExternalMemoryFn::name()),
			ExtensionOptions::Name(vk::KhrTimelineSemaphoreFn::name()),
			ExtensionOptions::SelectFirst(vec![
				vk::KhrExternalMemoryFdFn::name(),
				vk::KhrExternalMemoryWin32Fn::name(),
			]),
			ExtensionOptions::SelectFirst(vec![
				vk::KhrExternalSemaphoreFdFn::name(),
				vk::KhrExternalSemaphoreWin32Fn::name(),
			]),
			ExtensionOptions::Name(vk::ExtExternalMemoryHostFn::name()),
		];

		let mut physical_device_vk_12_features = vk::PhysicalDeviceVulkan12Features::builder()
			.timeline_semaphore(true)
			.build();

		let mut physical_device_features = vk::PhysicalDeviceFeatures2::builder()
			.features(vk::PhysicalDeviceFeatures::builder().build())
			.push_next(&mut physical_device_vk_12_features)
			.build();

		let physical_device_options = physical_device_options.unwrap_or_default();

		let avail_physical_devices = unsafe { vk_instance.instance.enumerate_physical_devices() }?;
		let (sel_physical_device, avail_extensions) =
			match avail_physical_devices.into_iter().find_map(|x| {
				Self::check_physical_device(
					x,
					&vk_instance.instance,
					Some(vk::API_VERSION_1_2),
					physical_device_options.vendor_id,
					physical_device_options.device_id,
					physical_device_options.device_uuid.as_ref(),
					physical_device_options.device_name.as_deref(),
					physical_device_options.device_type,
					Some(&extensions),
				)
			}) {
				Some(s) => s,
				None => return Err(vk::Result::ERROR_INITIALIZATION_FAILED),
			};

		let physical_device_queue_family_properties = unsafe {
			vk_instance
				.instance
				.get_physical_device_queue_family_properties(sel_physical_device)
		};
		let queue_family_index = match physical_device_queue_family_properties
			.into_iter()
			.position(|x| x.queue_flags.contains(vk::QueueFlags::GRAPHICS))
		{
			Some(index) => index as u32,
			None => return Err(vk::Result::ERROR_FEATURE_NOT_PRESENT),
		};

		let queue_device_info = vk::DeviceQueueCreateInfo::builder()
			.queue_family_index(queue_family_index)
			.queue_priorities(&[1.0])
			.build();

		let avail_extensions_c = avail_extensions
			.iter()
			.map(|x| x.as_ptr())
			.collect::<Vec<_>>();
		let device_create_info = vk::DeviceCreateInfo::builder()
			.enabled_extension_names(&avail_extensions_c)
			.queue_create_infos(&[queue_device_info])
			.push_next(&mut physical_device_features)
			.build();

		let vk_device = unsafe {
			vk_instance
				.instance
				.create_device(sel_physical_device, &device_create_info, None)
		}?;

		let vk_graphics_queue_index: u32 = 0;
		let vk_graphics_queue =
			unsafe { vk_device.get_device_queue(queue_family_index, vk_graphics_queue_index) };

		let vk_command_pool = Self::_create_command_pool(&vk_device, queue_family_index)?;
		let vk_command_buffer = Self::_allocate_command_buffer(
			&vk_device,
			vk_command_pool,
			vk::CommandBufferLevel::PRIMARY,
		)?;

		#[cfg(target_os = "linux")]
		let external_memory_fd =
			ash::extensions::khr::ExternalMemoryFd::new(&vk_instance.instance, &vk_device);

		Ok(VkDevice {
			device: vk_device,
			physical_device: sel_physical_device,
			graphics_queue: vk_graphics_queue,
			graphics_queue_family_index: queue_family_index,
			graphics_queue_index: vk_graphics_queue_index,
			import_only: false,
			command_pool: vk_command_pool,
			command_buffer: vk_command_buffer,
			external_memory_fd,
		})
	}

	pub fn import_vk(
		vk_instance: &VkInstance,
		vk_device: vk::Device,
		vk_physical_device: vk::PhysicalDevice,
		vk_graphics_queue: vk::Queue,
		vk_graphics_queue_family_index: u32,
		vk_graphics_queue_index: u32,
		import_only: bool,
	) -> Result<VkDevice, vk::Result> {
		let vk_device = unsafe { Device::load(vk_instance.instance.fp_v1_0(), vk_device) };

		let vk_command_pool =
			Self::_create_command_pool(&vk_device, vk_graphics_queue_family_index)?;
		let vk_command_buffer = Self::_allocate_command_buffer(
			&vk_device,
			vk_command_pool,
			vk::CommandBufferLevel::PRIMARY,
		)?;

		#[cfg(target_os = "linux")]
		let external_memory_fd =
			ash::extensions::khr::ExternalMemoryFd::new(&vk_instance.instance, &vk_device);

		Ok(VkDevice {
			device: vk_device,
			physical_device: vk_physical_device,
			graphics_queue: vk_graphics_queue,
			graphics_queue_family_index: vk_graphics_queue_family_index,
			graphics_queue_index: vk_graphics_queue_index,
			command_pool: vk_command_pool,
			command_buffer: vk_command_buffer,
			external_memory_fd,
			import_only,
		})
	}

	fn check_physical_device(
		physical_device: vk::PhysicalDevice,
		vk_instance: &Instance,
		api_version: Option<u32>,
		vendor_id: Option<u32>,
		device_id: Option<u32>,
		device_uuid: Option<&uuid::Uuid>,
		device_name: Option<&CStr>,
		device_type: Option<vk::PhysicalDeviceType>,
		device_extensions: Option<&[ExtensionOptions<'_>]>,
	) -> Option<(vk::PhysicalDevice, Vec<CString>)> {
		let props = unsafe { vk_instance.get_physical_device_properties(physical_device) };
		if let Some(api_version) = api_version {
			if props.api_version < api_version {
				return None;
			}
		}

		if let Some(vendor_id) = vendor_id {
			if vendor_id != props.vendor_id {
				return None;
			}
		}

		if let Some(device_id) = device_id {
			if device_id != props.device_id {
				return None;
			}
		}

		if let Some(device_uuid) = device_uuid {
			let cur_device_uuid = Self::get_gpu_device_uuid(vk_instance, physical_device);
			if cur_device_uuid != *device_uuid {
				return None;
			}
		}

		if let Some(device_name) = device_name {
			let phys_dev_name: &CStr = unsafe { VkEntry::to_cstr(&props.device_name) };
			if phys_dev_name != device_name {
				return None;
			}
		}

		if let Some(device_type) = device_type {
			if device_type != props.device_type {
				return None;
			}
		}

		let mut avail_extensions: Vec<CString> = vec![];
		if let Some(device_extensions) = device_extensions {
			let phys_device_extensions = unsafe {
				vk_instance
					.enumerate_device_extension_properties(physical_device)
					.unwrap()
			};

			let mut all_ext_avail = true;
			device_extensions.into_iter().for_each(|x| {
				match phys_device_extensions.iter().find(|&px| unsafe {
					match x {
						ExtensionOptions::Name(x) => *x == VkEntry::to_cstr(&px.extension_name),
						ExtensionOptions::SelectFirst(x) => {
							match x
								.iter()
								.find(|&&x| VkEntry::to_cstr(&px.extension_name) == x)
							{
								Some(_) => true,
								None => false,
							}
						}
					}
				}) {
					Some(ext_name) => {
						avail_extensions
							.push(unsafe { VkEntry::to_cstr(&ext_name.extension_name) }.into());
					}
					None => all_ext_avail = false,
				}
			});

			if !all_ext_avail {
				return None;
			}
		}

		return Some((physical_device, avail_extensions));
	}

	pub fn get_gpu_device_uuid(
		vk_instance: &Instance,
		physical_device: vk::PhysicalDevice,
	) -> uuid::Uuid {
		let mut uuid_props = vk::PhysicalDeviceVulkan11Properties::default();
		let mut props = vk::PhysicalDeviceProperties2::builder()
			.push_next(&mut uuid_props)
			.build();
		unsafe { vk_instance.get_physical_device_properties2(physical_device, &mut props) };

		let gpu_device_uuid = uuid::Uuid::from_bytes(uuid_props.device_uuid);
		//println!("UUID: {:}", gpu_device_uuid.to_string());
		gpu_device_uuid
	}

	pub fn get_gpu_vendor_device_ids(
		vk_instance: &Instance,
		physical_device: vk::PhysicalDevice,
	) -> (u32, u32) {
		let props = unsafe { vk_instance.get_physical_device_properties(physical_device) };
		(props.vendor_id, props.device_id)
	}

	pub fn get_external_memory_host_properties(
		vk_instance: &Instance,
		physical_device: vk::PhysicalDevice,
	) -> vk::PhysicalDeviceExternalMemoryHostPropertiesEXT {
		let mut external_host_props = vk::PhysicalDeviceExternalMemoryHostPropertiesEXT::default();
		let mut prop = vk::PhysicalDeviceProperties2::builder()
			.push_next(&mut external_host_props)
			.build();

		unsafe { vk_instance.get_physical_device_properties2(physical_device, &mut prop) };

		external_host_props
	}

	fn _create_command_pool(
		vk_device: &Device,
		vk_graphics_queue_family_index: u32,
	) -> Result<vk::CommandPool, vk::Result> {
		let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
			.flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
			.queue_family_index(vk_graphics_queue_family_index)
			.build();

		let command_pool =
			unsafe { vk_device.create_command_pool(&command_pool_create_info, None) }?;

		Ok(command_pool)
	}

	fn _destroy_command_pool(&self, command_pool: vk::CommandPool) {
		unsafe { self.device.destroy_command_pool(command_pool, None) };
	}

	fn _allocate_command_buffer(
		vk_device: &Device,
		command_pool: vk::CommandPool,
		level: vk::CommandBufferLevel,
	) -> Result<vk::CommandBuffer, vk::Result> {
		let allocate_info = vk::CommandBufferAllocateInfo::builder()
			.command_pool(command_pool)
			.command_buffer_count(1)
			.level(level)
			.build();

		let command_buffer = unsafe { vk_device.allocate_command_buffers(&allocate_info) }?;
		Ok(command_buffer[0])
	}

	fn _free_command_buffer(
		&self,
		command_pool: &vk::CommandPool,
		command_buffer: vk::CommandBuffer,
	) {
		unsafe {
			self.device
				.free_command_buffers(*command_pool, &[command_buffer])
		};
	}

	pub fn create_fence(
		&self,
		fence_info: Option<vk::FenceCreateInfo>,
	) -> Result<vk::Fence, vk::Result> {
		let fence_info = match fence_info {
			Some(s) => s,
			None => vk::FenceCreateInfo::default(),
		};
		let fence = unsafe { self.device.create_fence(&fence_info, None) }?;
		return Ok(fence);
	}

	pub fn destroy_fence(&self, fence: vk::Fence) {
		unsafe { self.device.destroy_fence(fence, None) };
	}

	fn _create_buffer(
		vk_device: &Device,
		create_info: &vk::BufferCreateInfo,
	) -> Result<VkBuffer, vk::Result> {
		let handle = unsafe { vk_device.create_buffer(create_info, None) }?;
		Ok(VkBuffer { handle })
	}

	pub fn create_buffer(
		&self,
		create_info: &vk::BufferCreateInfo,
	) -> Result<VkBuffer, vk::Result> {
		Self::_create_buffer(&self.device, create_info)
	}

	pub fn destroy_buffer(&self, vk_buffer: VkBuffer) {
		unsafe { self.device.destroy_buffer(vk_buffer.handle, None) }
		std::mem::forget(vk_buffer)
	}

	pub fn immediate_submit_with_fence<F: FnOnce(vk::CommandBuffer) -> Result<(), vk::Result>>(
		&self,
		command_buffer: vk::CommandBuffer,
		fill_cmd_buf_fcn: F,
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
		fence: vk::Fence,
	) -> Result<(), vk::Result> {
		let cmd_begin_info = vk::CommandBufferBeginInfo::builder()
			.flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
			.build();
		unsafe {
			self.device
				.begin_command_buffer(command_buffer, &cmd_begin_info)
		}?;

		fill_cmd_buf_fcn(command_buffer).unwrap();

		unsafe { self.device.end_command_buffer(command_buffer) }?;

		// For some reason, optimization breaks the builder() function. Instead, manually build submit_info
		// let submit_info = vk::SubmitInfo::builder()
		// 	.command_buffers(&[command_buffer])
		// 	.signal_semaphores(signal_semaphores)
		// 	.wait_semaphores(wait_semaphores)
		// 	.build();
		let submit_info = vk::SubmitInfo {
			command_buffer_count: 1,
			p_command_buffers: &command_buffer as *const _,
			signal_semaphore_count: signal_semaphores.len() as u32,
			p_signal_semaphores: signal_semaphores.as_ref() as *const _ as *const _,
			wait_semaphore_count: wait_semaphores.len() as u32,
			p_wait_semaphores: wait_semaphores.as_ref() as *const _ as *const _,
			..Default::default()
		};

		unsafe {
			self.device
				.queue_submit(self.graphics_queue, &[submit_info], fence)?;
			self.device
				.wait_for_fences(&[fence], true, 1000 * 1000 * 1000)?;
			self.device.reset_fences(&[fence])?;
		};

		Ok(())
	}

	pub fn immediate_submit<F: FnOnce(vk::CommandBuffer) -> Result<(), vk::Result>>(
		&self,
		command_buffer: vk::CommandBuffer,
		fill_cmd_buf_fcn: F,
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
	) -> Result<(), vk::Result> {
		let fence = self.create_fence(None)?;

		let res = self.immediate_submit_with_fence(
			command_buffer,
			fill_cmd_buf_fcn,
			wait_semaphores,
			signal_semaphores,
			fence,
		);

		// Destroy fence before propagating result
		self.destroy_fence(fence);

		res?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use std::ffi::CStr;

	use super::VkDevice;
	use crate::vk_instance::VkInstance;

	fn _init_vk_instance() -> VkInstance {
		VkInstance::new(None, CStr::from_bytes_with_nul(b"Name\0").unwrap(), true).unwrap()
	}

	fn _init_vk_device(vk_instance: &VkInstance) -> VkDevice {
		VkDevice::new(vk_instance, None).unwrap()
	}

	#[test]
	fn vk_device_new() {
		let vk_instance = _init_vk_instance();
		let _vk_device = _init_vk_device(&vk_instance);
	}

	#[test]
	fn vk_device_fence_new() {
		let vk_instance = _init_vk_instance();
		let vk_device = _init_vk_device(&vk_instance);

		let fence = vk_device.create_fence(None).unwrap();
		vk_device.destroy_fence(fence);
	}

	#[test]
	fn vk_immediate_submit() {
		let vk_instance = _init_vk_instance();
		let vk_device = _init_vk_device(&vk_instance);

		vk_device
			.immediate_submit(vk_device.command_buffer, |_x| Ok(()), &[], &[])
			.unwrap();
	}
}
