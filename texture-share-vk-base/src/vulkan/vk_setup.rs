use cxx::{type_id, ExternType};
use libc::c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkInstance {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkInstance {
	type Id = type_id!("VkInstance");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkDevice {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkDevice {
	type Id = type_id!("VkDevice");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkPhysicalDevice {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkPhysicalDevice {
	type Id = type_id!("VkPhysicalDevice");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkQueue {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkQueue {
	type Id = type_id!("VkQueue");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkCommandPool {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkCommandPool {
	type Id = type_id!("VkCommandPool");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkCommandBuffer {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkCommandBuffer {
	type Id = type_id!("VkCommandBuffer");
	type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkFence {
	_ptr: *mut c_void,
}

unsafe impl ExternType for VkFence {
	type Id = type_id!("VkFence");
	type Kind = cxx::kind::Trivial;
}

#[cxx::bridge]
pub mod ffi {
	unsafe extern "C++" {
		include!("wrapper/vk_setup_wrapper.h");

		type VkInstance = super::VkInstance;
		type VkDevice = super::VkDevice;
		type VkPhysicalDevice = super::VkPhysicalDevice;
		type VkQueue = super::VkQueue;
		type VkCommandPool = super::VkCommandPool;
		type VkCommandBuffer = super::VkCommandBuffer;
		type VkFence = super::VkFence;

		type ExternalHandleVk;

		#[rust_name = "VkSetup"]
		type VkSetupWrapper;

		fn vk_setup_new() -> UniquePtr<VkSetup>;
		fn initialize_vulkan(self: Pin<&mut VkSetup>);
		fn import_vulkan(
			self: Pin<&mut VkSetup>,
			instance: VkInstance,
			device: VkDevice,
			physical_device: VkPhysicalDevice,
			graphics_queue: VkQueue,
			graphics_queue_index: u32,
		);
		fn import_vulkan_as_owned(
			self: Pin<&mut VkSetup>,
			instance: VkInstance,
			device: VkDevice,
			physical_device: VkPhysicalDevice,
			graphics_queue: VkQueue,
			graphics_queue_index: u32,
		);

		fn cleanup_vulkan(self: Pin<&mut VkSetup>);

		fn is_vulkan_initialized(self: &VkSetup) -> bool;

		fn get_vk_instance(self: &VkSetup) -> VkInstance;
		fn get_vk_device(self: &VkSetup) -> VkDevice;
		fn get_vk_physical_device(self: &VkSetup) -> VkPhysicalDevice;
		fn get_vk_queue(self: &VkSetup) -> VkQueue;
		fn get_vk_queue_index(self: &VkSetup) -> u32;
		fn get_vk_command_pool(self: &VkSetup) -> VkCommandPool;
		fn get_vk_command_buffer(self: &VkSetup) -> VkCommandBuffer;

		fn get_external_handle_info(self: &VkSetup) -> &ExternalHandleVk;

		fn create_vk_fence(self: Pin<&mut VkSetup>) -> VkFence;
		fn destroy_vk_fence(self: Pin<&mut VkSetup>, fence: VkFence);
	}
}

#[cfg(test)]
mod tests {
	use cxx::UniquePtr;

	use super::ffi::{vk_setup_new, VkSetup};

	#[test]
	fn vk_setup_test_new() {
		vk_setup_new();
	}

	fn _init_vulkan() -> UniquePtr<VkSetup> {
		let mut vk_setup = vk_setup_new();
		vk_setup.as_mut().unwrap().initialize_vulkan();

		vk_setup
	}

	#[test]
	fn vk_setup_initialize_vulkan() {
		let _ = _init_vulkan();
	}

	#[test]
	fn vk_setup_cleanup() {
		let mut vk_setup = _init_vulkan();
		vk_setup.as_mut().unwrap().cleanup_vulkan();
	}

	#[test]
	fn vk_setup_init_multiple_instances() {
		// Note: This fails if debug layer is enabled. It's a problem with VkBootstrap from what I can tell
		let _inst1 = _init_vulkan();
		let _inst2 = _init_vulkan();
	}

	#[test]
	fn vk_setup_check_instance() {
		let mut vk_setup = vk_setup_new();
		assert_eq!(vk_setup.as_ref().unwrap().is_vulkan_initialized(), false);

		vk_setup.as_mut().unwrap().initialize_vulkan();
		assert_eq!(vk_setup.as_ref().unwrap().is_vulkan_initialized(), true);
	}

	#[test]
	fn vk_setup_import() {
		let mut vk_setup_own = vk_setup_new();
		let mut vk_setup_import = vk_setup_new();

		vk_setup_own.as_mut().unwrap().initialize_vulkan();
		vk_setup_import.as_mut().unwrap().import_vulkan(
			vk_setup_own.get_vk_instance(),
			vk_setup_own.get_vk_device(),
			vk_setup_own.get_vk_physical_device(),
			vk_setup_own.get_vk_queue(),
			vk_setup_own.get_vk_queue_index(),
		);

		assert_eq!(
			vk_setup_own.get_vk_instance(),
			vk_setup_own.get_vk_instance()
		);
		assert_eq!(vk_setup_own.get_vk_device(), vk_setup_own.get_vk_device());
		assert_eq!(
			vk_setup_own.get_vk_physical_device(),
			vk_setup_own.get_vk_physical_device()
		);
		assert_eq!(vk_setup_own.get_vk_queue(), vk_setup_own.get_vk_queue());
		assert_eq!(
			vk_setup_own.get_vk_queue_index(),
			vk_setup_own.get_vk_queue_index()
		);

		vk_setup_import.as_mut().unwrap().cleanup_vulkan();
		vk_setup_own.as_mut().unwrap().cleanup_vulkan();
	}

	#[test]
	fn vk_setup_fence() {
		let mut vk_setup = vk_setup_new();
		vk_setup.as_mut().unwrap().initialize_vulkan();

		let vk_fence = vk_setup.as_mut().unwrap().create_vk_fence();
		vk_setup.as_mut().unwrap().destroy_vk_fence(vk_fence);
	}
}
