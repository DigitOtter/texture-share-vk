use crate::vk_setup::VkSetup;

use ash::{self, vk};
use std::boxed::Box;
use std::{ffi::CStr, pin::Pin};

type VkInstance = vk::Instance;
type VkDevice = vk::Device;
type VkPhysicalDevice = vk::PhysicalDevice;
type VkQueue = vk::Queue;

pub unsafe fn vk_setup_init_c() -> *mut VkSetup {
	let pvk_setup = Box::new(VkSetup::new(CStr::from_bytes_with_nul(b"VkSetup").unwrap()).unwrap());
	Box::into_raw(pvk_setup)
}

pub unsafe fn vk_setup_c_as_mut<'a>(vk_setup: &'a *mut VkSetup) -> Pin<&'a mut VkSetup> {
	Pin::new_unchecked(vk_setup.as_mut().unwrap_unchecked())
}

pub unsafe fn vk_setup_from_c(vk_setup: *mut VkSetup) -> Box<VkSetup> {
	Box::from_raw(vk_setup)
}

// unsafe fn vk_setup_as_mut<'a>(vk_setup: &'a *mut VkSetup) -> Pin<&'a mut VkSetup> {
// 	unsafe { Pin::new_unchecked(vk_setup.as_mut().unwrap()) }
// }

#[no_mangle]
extern "C" fn vk_setup_new() -> *mut VkSetup {
	unsafe { vk_setup_init_c() }
}

#[no_mangle]
extern "C" fn vk_setup_destroy(vk_setup: *mut VkSetup) {
	// Expplicitly drop VkSetup
	let pvk_setup = unsafe { vk_setup_from_c(vk_setup) };
	std::mem::drop(pvk_setup);
}

#[no_mangle]
extern "C" fn vk_setup_initialize_vulkan(vk_setup: *mut VkSetup) {
	unsafe { vk_setup_c_as_mut(&vk_setup) };
}

#[no_mangle]
extern "C" fn vk_setup_import_vulkan(
	vk_setup: *mut VkSetup,
	instance: VkInstance,
	device: VkDevice,
	physical_device: VkPhysicalDevice,
	graphics_queue: VkQueue,
	graphics_queue_family_index: u32,
	import_only: bool,
) {
	let mut vk_setup = unsafe { vk_setup_c_as_mut(&vk_setup) };
	*vk_setup = VkSetup::import_vk(
		None,
		instance,
		device,
		physical_device,
		graphics_queue,
		graphics_queue_family_index,
		0,
		import_only,
	)
	.unwrap();
}

#[no_mangle]
extern "C" fn vk_setup_new_import_vulkan(
	instance: VkInstance,
	device: VkDevice,
	physical_device: VkPhysicalDevice,
	graphics_queue: VkQueue,
	graphics_queue_family_index: u32,
	import_only: bool,
) -> *mut VkSetup {
	let vk_setup = Box::new(
		VkSetup::import_vk(
			None,
			instance,
			device,
			physical_device,
			graphics_queue,
			graphics_queue_family_index,
			0,
			import_only,
		)
		.unwrap(),
	);

	Box::into_raw(vk_setup)
}

// #[no_mangle]
// extern "C" fn vk_setup_cleanup_vulkan(vk_setup: *mut VkSetup) {
// 	unsafe { vk_setup_as_mut(&vk_setup) }.cleanup_vulkan();
// }

// #[no_mangle]
// extern "C" fn vk_setup_is_vulkan_initialized(vk_setup: *mut VkSetup) {
// 	unsafe { vk_setup.as_ref().unwrap() }.is_vulkan_initialized();
// }
