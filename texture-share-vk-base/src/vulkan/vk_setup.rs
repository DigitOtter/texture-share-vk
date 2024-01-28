use std::{
	borrow::BorrowMut,
	cmp::Ordering,
	default,
	error::Error,
	ffi::{CStr, CString},
	marker::PhantomData,
};

use ash::{vk, Device, Entry, Instance, LoadingError, RawPtr};
use cxx::{type_id, ExternType};
use libc::{c_char, c_void};

pub struct VkSetup {
	entry: Box<Entry>,
	pub vk_instance: Instance,
	pub vk_device: Device,
	pub vk_physical_device: vk::PhysicalDevice,

	pub vk_graphics_queue: vk::Queue,
	pub vk_graphics_queue_family_index: u32,
	pub vk_graphics_queue_index: u32,

	pub vk_command_pool: vk::CommandPool,
	pub vk_command_buffer: vk::CommandBuffer,

	#[cfg(target_os = "linux")]
	pub external_memory_fd: ash::extensions::khr::ExternalMemoryFd,

	import_only: bool,
}

pub struct VkCommandPoola<'a> {
	handle: vk::CommandPool,
	phantom_dev: PhantomData<&'a VkSetup>,
}

pub struct VkCommandBuffera<'a> {
	pub handle: vk::CommandBuffer,
	phantom_pool: PhantomData<&'a VkCommandPoola<'a>>,
}

pub struct VkSemaphore<'a> {
	pub handle: vk::Semaphore,
	phantom_dev: PhantomData<&'a VkSetup>,
}

pub struct VkFencea {
	pub handle: vk::Fence,
	//_phantom_dev: PhantomData<&'a VkSetup>,
}

enum ExtensionOptions<'a> {
	Name(&'a CStr),
	SelectFirst(Vec<&'a CStr>),
}

impl Drop for VkSetup {
	fn drop(&mut self) {
		unsafe {
			self.vk_device.device_wait_idle().unwrap();

			self._free_command_buffer(&self.vk_command_pool, self.vk_command_buffer);
			self._destroy_command_pool(self.vk_command_pool);
			if !self.import_only {
				self.vk_device.destroy_device(None);
				self.vk_instance.destroy_instance(None)
			};
		}
	}
}

impl Drop for VkCommandPoola<'_> {
	fn drop(&mut self) {
		println!("Warning: VkCommandPool should be manually destroyed, not dropped");
	}
}

impl Drop for VkCommandBuffera<'_> {
	fn drop(&mut self) {
		println!("Warning: VkCommandBuffer should be manually destroyed, not dropped");
	}
}

impl Drop for VkSemaphore<'_> {
	fn drop(&mut self) {
		println!("Warning: VkSemaphore should be manually destroyed, not dropped");
	}
}

impl Drop for VkFencea {
	fn drop(&mut self) {
		println!("Warning: VkFence should be manually destroyed, not dropped");
	}
}

impl VkSetup {
	unsafe fn to_cstr(buf: &[i8]) -> &CStr {
		let buf: &[u8] = &*(buf as *const [i8] as *const [u8]);
		let cstr = CStr::from_bytes_until_nul(buf).unwrap();
		cstr
	}

	fn check_physical_device(
		physical_device: vk::PhysicalDevice,
		vk_instance: &Instance,
		api_version: Option<u32>,
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

		if let Some(device_name) = device_name {
			let phys_dev_name: &CStr = unsafe { Self::to_cstr(&props.device_name) };
			if phys_dev_name != device_name {
				return None;
			}
		}

		if let Some(device_type) = device_type {
			if device_type == props.device_type {
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
						ExtensionOptions::Name(x) => *x == Self::to_cstr(&px.extension_name),
						ExtensionOptions::SelectFirst(x) => {
							match x.iter().find(|&&x| Self::to_cstr(&px.extension_name) == x) {
								Some(_) => true,
								None => false,
							}
						}
					}
				}) {
					Some(ext_name) => {
						avail_extensions
							.push(unsafe { Self::to_cstr(&ext_name.extension_name) }.into());
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

	pub fn import_vk(
		entry: Option<Box<Entry>>,
		vk_instance: vk::Instance,
		vk_device: vk::Device,
		vk_physical_device: vk::PhysicalDevice,
		vk_graphics_queue: vk::Queue,
		vk_graphics_queue_family_index: u32,
		vk_graphics_queue_index: u32,
		import_only: bool,
	) -> Result<VkSetup, vk::Result> {
		let entry = entry.unwrap_or(Box::new(unsafe {
			match Entry::load() {
				Ok(o) => o,
				_ => return Err(vk::Result::NOT_READY),
			}
		}));

		let vk_instance = unsafe { Instance::load(entry.static_fn(), vk_instance) };
		let vk_device = unsafe { Device::load(vk_instance.fp_v1_0(), vk_device) };

		let vk_command_pool =
			Self::_create_command_pool(&vk_device, vk_graphics_queue_family_index)?;
		let vk_command_buffer = Self::_allocate_command_buffer(
			&vk_device,
			vk_command_pool,
			vk::CommandBufferLevel::PRIMARY,
		)?;

		#[cfg(target_os = "linux")]
		let external_memory_fd = ash::extensions::khr::ExternalMemoryFd::new(&vk_instance, &vk_device);

		Ok(VkSetup {
			entry,
			vk_instance,
			vk_device,
			vk_physical_device,
			vk_graphics_queue,
			vk_graphics_queue_family_index,
			vk_graphics_queue_index,
			vk_command_pool,
			vk_command_buffer,
			external_memory_fd,
			import_only,
		})
	}

	pub fn new(vk_instance_name: &CStr) -> Result<VkSetup, vk::Result> {
		let entry: Box<Entry> = Box::new(unsafe {
			match Entry::load() {
				Ok(o) => o,
				_ => return Err(vk::Result::NOT_READY),
			}
		});

		let ext_properties = entry.enumerate_instance_extension_properties(None)?;
		let extensions = [
			vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr(),
			vk::KhrExternalSemaphoreCapabilitiesFn::name().as_ptr(),
			vk::KhrExternalMemoryCapabilitiesFn::name().as_ptr(),
		];

		let app_info = vk::ApplicationInfo {
			api_version: vk::make_api_version(0, 1, 2, 0),
			p_application_name: vk_instance_name.as_ptr(),
			..Default::default()
		};

		let create_info = vk::InstanceCreateInfo {
			p_application_info: &app_info,
			enabled_extension_count: extensions.len() as u32,
			pp_enabled_extension_names: extensions.as_ptr(),
			..Default::default()
		};

		let vk_instance = unsafe { entry.create_instance(&create_info, None) }?;

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
		];

		let mut physical_device_vk_12_features = vk::PhysicalDeviceVulkan12Features::builder()
			.timeline_semaphore(true)
			.build();

		let mut physical_device_features = vk::PhysicalDeviceFeatures2::builder()
			.features(vk::PhysicalDeviceFeatures::builder().build())
			.push_next(&mut physical_device_vk_12_features)
			.build();

		let avail_physical_devices = unsafe { vk_instance.enumerate_physical_devices() }?;
		let (sel_physical_device, avail_extensions) =
			match avail_physical_devices.into_iter().find_map(|x| {
				Self::check_physical_device(
					x,
					&vk_instance,
					Some(vk::API_VERSION_1_2),
					None,
					None,
					Some(&extensions),
				)
			}) {
				Some(s) => s,
				None => return Err(vk::Result::ERROR_INITIALIZATION_FAILED),
			};

		let physical_device_queue_family_properties =
			unsafe { vk_instance.get_physical_device_queue_family_properties(sel_physical_device) };
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

		let vk_device =
			unsafe { vk_instance.create_device(sel_physical_device, &device_create_info, None) }?;

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
		let external_memory_fd = ash::extensions::khr::ExternalMemoryFd::new(&vk_instance, &vk_device);

		Ok(VkSetup {
			entry,
			vk_instance,
			vk_device,
			vk_physical_device: sel_physical_device,
			vk_graphics_queue,
			vk_graphics_queue_family_index: queue_family_index,
			vk_graphics_queue_index,
			vk_command_pool,
			vk_command_buffer,
			external_memory_fd,
			import_only: false,
		})
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

	pub fn create_command_pool<'a: 'b, 'b>(&'a self) -> Result<VkCommandPoola<'b>, vk::Result> {
		let command_pool =
			Self::_create_command_pool(&self.vk_device, self.vk_graphics_queue_family_index)?;

		Ok(VkCommandPoola {
			handle: command_pool,
			phantom_dev: PhantomData,
		})
	}

	fn _destroy_command_pool(&self, command_pool: vk::CommandPool) {
		unsafe { self.vk_device.destroy_command_pool(command_pool, None) };
	}

	pub fn destroy_command_pool(&self, command_pool: VkCommandPoola) {
		self._destroy_command_pool(command_pool.handle);
		std::mem::forget(command_pool);
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

	pub fn allocate_command_buffer<'a: 'b, 'b>(
		&self,
		command_pool: &'a VkCommandPoola,
		level: vk::CommandBufferLevel,
	) -> Result<VkCommandBuffera<'b>, vk::Result> {
		let command_buffer =
			Self::_allocate_command_buffer(&self.vk_device, command_pool.handle, level)?;

		Ok(VkCommandBuffera {
			handle: command_buffer,
			phantom_pool: PhantomData,
		})
	}

	fn _free_command_buffer(
		&self,
		command_pool: &vk::CommandPool,
		command_buffer: vk::CommandBuffer,
	) {
		unsafe {
			self.vk_device
				.free_command_buffers(*command_pool, &[command_buffer])
		};
	}

	pub fn free_command_buffer(
		&self,
		command_pool: &VkCommandPoola<'_>,
		command_buffer: VkCommandBuffera<'_>,
	) {
		self._free_command_buffer(&command_pool.handle, command_buffer.handle);
		std::mem::forget(command_buffer);
	}

	pub fn create_semaphore(
		&self,
		create_info: Option<vk::SemaphoreCreateInfo>,
	) -> Result<VkSemaphore, vk::Result> {
		let create_info = match create_info {
			Some(s) => s,
			None => vk::SemaphoreCreateInfo::default(),
		};
		let semaphore = unsafe { self.vk_device.create_semaphore(&create_info, None) }?;

		return Ok(VkSemaphore {
			handle: semaphore,
			phantom_dev: PhantomData,
		});
	}

	pub fn destroy_semaphore(&self, semaphore: VkSemaphore) {
		unsafe { self.vk_device.destroy_semaphore(semaphore.handle, None) };
		std::mem::forget(semaphore);
	}

	pub fn create_fence(
		&self,
		fence_info: Option<vk::FenceCreateInfo>,
	) -> Result<VkFencea, vk::Result> {
		let fence_info = match fence_info {
			Some(s) => s,
			None => vk::FenceCreateInfo::default(),
		};
		let fence = unsafe { self.vk_device.create_fence(&fence_info, None) }?;
		return Ok(VkFencea {
			handle: fence,
			//_phantom_dev: PhantomData,
		});
	}

	pub fn destroy_fence(&self, fence: VkFencea) {
		unsafe { self.vk_device.destroy_fence(fence.handle, None) };
		std::mem::forget(fence);
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
			self.vk_device
				.begin_command_buffer(command_buffer, &cmd_begin_info)
		}?;

		fill_cmd_buf_fcn(command_buffer).unwrap();

		unsafe { self.vk_device.end_command_buffer(command_buffer) }?;

		let submit_info = vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.signal_semaphores(signal_semaphores)
			.wait_semaphores(wait_semaphores)
			.build();

		unsafe {
			self.vk_device
				.queue_submit(self.vk_graphics_queue, &[submit_info], fence)?;
			self.vk_device
				.wait_for_fences(&[fence], true, 1000 * 1000)?;
			self.vk_device.reset_fences(&[fence])?;
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

		self.immediate_submit_with_fence(
			command_buffer,
			fill_cmd_buf_fcn,
			wait_semaphores,
			signal_semaphores,
			fence.handle,
		)
		.map(|x| {
			// Destroy fence before propagating result
			self.destroy_fence(fence);
			x
		})?;

		Ok(())
	}
}

#[cfg(test)]
mod testsa {
	use std::ffi::CStr;

	use ash::vk;

	use super::VkSetup;

	fn _init_vk_setup() -> VkSetup {
		VkSetup::new(CStr::from_bytes_with_nul(b"Name\0").unwrap()).unwrap()
	}

	#[test]
	fn vk_setup_test_new() {
		let _vk_setup = _init_vk_setup();
	}

	#[test]
	fn vk_command_pool_new() {
		let vk_setup = _init_vk_setup();
		let cmd_pool = vk_setup.create_command_pool().unwrap();
		vk_setup.destroy_command_pool(cmd_pool);
	}

	#[test]
	fn vk_command_buffer_new() {
		let vk_setup = _init_vk_setup();
		let cmd_pool = vk_setup.create_command_pool().unwrap();
		let cmd_buffer = vk_setup
			.allocate_command_buffer(&cmd_pool, vk::CommandBufferLevel::PRIMARY)
			.unwrap();
		vk_setup.free_command_buffer(&cmd_pool, cmd_buffer);
		vk_setup.destroy_command_pool(cmd_pool);
	}

	#[test]
	fn vk_semaphore_new() {
		let vk_setup = _init_vk_setup();
		let semaphore = vk_setup.create_semaphore(None).unwrap();
		vk_setup.destroy_semaphore(semaphore);
	}

	#[test]
	fn vk_fence_new() {
		let vk_setup = _init_vk_setup();
		let fence = vk_setup.create_fence(None).unwrap();
		vk_setup.destroy_fence(fence);
	}

	#[test]
	fn vk_immediate_submit() {
		let vk_setup = _init_vk_setup();
		let cmd_pool = vk_setup.create_command_pool().unwrap();
		let cmd_buffer = vk_setup
			.allocate_command_buffer(&cmd_pool, vk::CommandBufferLevel::PRIMARY)
			.unwrap();

		vk_setup
			.immediate_submit(cmd_buffer.handle, |_x| Ok(()), &[], &[])
			.unwrap();

		vk_setup.free_command_buffer(&cmd_pool, cmd_buffer);
		vk_setup.destroy_command_pool(cmd_pool);
	}
}

// #[cfg(test)]
// mod tests {
// 	use cxx::UniquePtr;

// 	use super::ffi::{vk_setup_new, VkSetup};

// 	#[test]
// 	fn vk_setup_test_new() {
// 		vk_setup_new();
// 	}

// 	fn _init_vulkan() -> UniquePtr<VkSetup> {
// 		let mut vk_setup = vk_setup_new();
// 		vk_setup.as_mut().unwrap().initialize_vulkan();

// 		vk_setup
// 	}

// 	#[test]
// 	fn vk_setup_initialize_vulkan() {
// 		let _ = _init_vulkan();
// 	}

// 	#[test]
// 	fn vk_setup_cleanup() {
// 		let mut vk_setup = _init_vulkan();
// 		vk_setup.as_mut().unwrap().cleanup_vulkan();
// 	}

// 	#[test]
// 	fn vk_setup_init_multiple_instances() {
// 		// Note: This fails if debug layer is enabled. It's a problem with VkBootstrap from what I can tell
// 		let _inst1 = _init_vulkan();
// 		let _inst2 = _init_vulkan();
// 	}

// 	#[test]
// 	fn vk_setup_check_instance() {
// 		let mut vk_setup = vk_setup_new();
// 		assert_eq!(vk_setup.as_ref().unwrap().is_vulkan_initialized(), false);

// 		vk_setup.as_mut().unwrap().initialize_vulkan();
// 		assert_eq!(vk_setup.as_ref().unwrap().is_vulkan_initialized(), true);
// 	}

// 	#[test]
// 	fn vk_setup_import() {
// 		let mut vk_setup_own = vk_setup_new();
// 		let mut vk_setup_import = vk_setup_new();

// 		vk_setup_own.as_mut().unwrap().initialize_vulkan();
// 		vk_setup_import.as_mut().unwrap().import_vulkan(
// 			vk_setup_own.get_vk_instance(),
// 			vk_setup_own.get_vk_device(),
// 			vk_setup_own.get_vk_physical_device(),
// 			vk_setup_own.get_vk_queue(),
// 			vk_setup_own.get_vk_queue_index(),
// 		);

// 		assert_eq!(
// 			vk_setup_own.get_vk_instance(),
// 			vk_setup_own.get_vk_instance()
// 		);
// 		assert_eq!(vk_setup_own.get_vk_device(), vk_setup_own.get_vk_device());
// 		assert_eq!(
// 			vk_setup_own.get_vk_physical_device(),
// 			vk_setup_own.get_vk_physical_device()
// 		);
// 		assert_eq!(vk_setup_own.get_vk_queue(), vk_setup_own.get_vk_queue());
// 		assert_eq!(
// 			vk_setup_own.get_vk_queue_index(),
// 			vk_setup_own.get_vk_queue_index()
// 		);

// 		vk_setup_import.as_mut().unwrap().cleanup_vulkan();
// 		vk_setup_own.as_mut().unwrap().cleanup_vulkan();
// 	}

// 	#[test]
// 	fn vk_setup_fence() {
// 		let mut vk_setup = vk_setup_new();
// 		vk_setup.as_mut().unwrap().initialize_vulkan();

// 		let vk_fence = vk_setup.as_mut().unwrap().create_vk_fence();
// 		vk_setup.as_mut().unwrap().destroy_vk_fence(vk_fence);
// 	}
// }
